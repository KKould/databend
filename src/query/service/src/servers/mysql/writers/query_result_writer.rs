// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use common_base::base::tokio::io::AsyncWrite;
use common_datavalues::prelude::TypeID;
use common_datavalues::remove_nullable;
use common_datavalues::DataField;
use common_datavalues::DataSchemaRef;
use common_datavalues::DataSchemaRefExt;
use common_datavalues::DataType;
use common_datavalues::DataValue;
use common_datavalues::DateConverter;
use common_datavalues::TypeSerializer;
use common_exception::ErrorCode;
use common_exception::Result;
use common_exception::ABORT_QUERY;
use common_exception::ABORT_SESSION;
use common_io::prelude::FormatSettings;
use common_streams::DataBlockStream;
use common_streams::SendableDataBlockStream;
use futures_util::StreamExt;
use opensrv_mysql::*;
use tracing::error;

pub struct QueryResult {
    blocks: SendableDataBlockStream,
    extra_info: String,
    has_result_set: bool,
    schema: DataSchemaRef,
}

impl QueryResult {
    pub fn create(
        blocks: SendableDataBlockStream,
        extra_info: String,
        has_result_set: bool,
        schema: DataSchemaRef,
    ) -> QueryResult {
        QueryResult {
            blocks,
            extra_info,
            has_result_set,
            schema,
        }
    }

    pub fn default() -> QueryResult {
        let schema = DataSchemaRefExt::create(vec![]);
        QueryResult {
            blocks: DataBlockStream::create(schema.clone(), None, vec![]).boxed(),
            extra_info: String::from(""),
            has_result_set: false,
            schema,
        }
    }
}

pub struct DFQueryResultWriter<'a, W: AsyncWrite + Send + Unpin> {
    inner: Option<QueryResultWriter<'a, W>>,
}

impl<'a, W: AsyncWrite + Send + Unpin> DFQueryResultWriter<'a, W> {
    pub fn create(inner: QueryResultWriter<'a, W>) -> DFQueryResultWriter<'a, W> {
        DFQueryResultWriter::<'a, W> { inner: Some(inner) }
    }

    pub async fn write(
        &mut self,
        query_result: Result<QueryResult>,
        format: &FormatSettings,
    ) -> Result<()> {
        if let Some(writer) = self.inner.take() {
            match query_result {
                Ok(query_result) => Self::ok(query_result, writer, format).await?,
                Err(error) => Self::err(&error, writer).await?,
            }
        }
        Ok(())
    }

    async fn ok(
        mut query_result: QueryResult,
        dataset_writer: QueryResultWriter<'a, W>,
        format: &FormatSettings,
    ) -> Result<()> {
        // XXX: num_columns == 0 may is error?
        let default_response = OkResponse {
            info: query_result.extra_info,
            ..Default::default()
        };

        // if (!query_result.has_result_set) || (query_result.schema.num_fields() == 0) {
        if !query_result.has_result_set {
            dataset_writer.completed(default_response).await?;
            eprintln!(
                "writing HERE, has_result_set {}, num_fields {}",
                query_result.has_result_set,
                query_result.schema.num_fields()
            );
            return Ok(());
        }

        fn convert_field_type(field: &DataField) -> Result<ColumnType> {
            match remove_nullable(field.data_type()).data_type_id() {
                TypeID::Int8 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::Int16 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::Int32 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::Int64 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::UInt8 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::UInt16 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::UInt32 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::UInt64 => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::Float32 => Ok(ColumnType::MYSQL_TYPE_FLOAT),
                TypeID::Float64 => Ok(ColumnType::MYSQL_TYPE_FLOAT),
                TypeID::String => Ok(ColumnType::MYSQL_TYPE_VARCHAR),
                TypeID::Boolean => Ok(ColumnType::MYSQL_TYPE_SHORT),
                TypeID::Date => Ok(ColumnType::MYSQL_TYPE_DATE),
                TypeID::Timestamp => Ok(ColumnType::MYSQL_TYPE_DATETIME),
                TypeID::Null => Ok(ColumnType::MYSQL_TYPE_NULL),
                TypeID::Interval => Ok(ColumnType::MYSQL_TYPE_LONG),
                TypeID::Array => Ok(ColumnType::MYSQL_TYPE_VARCHAR),
                TypeID::Struct => Ok(ColumnType::MYSQL_TYPE_VARCHAR),
                TypeID::Variant => Ok(ColumnType::MYSQL_TYPE_VARCHAR),
                TypeID::VariantArray => Ok(ColumnType::MYSQL_TYPE_VARCHAR),
                TypeID::VariantObject => Ok(ColumnType::MYSQL_TYPE_VARCHAR),
                _ => Err(ErrorCode::UnImplement(format!(
                    "Unsupported column type:{:?}",
                    field.data_type()
                ))),
            }
        }

        fn make_column_from_field(field: &DataField) -> Result<Column> {
            convert_field_type(field).map(|column_type| Column {
                table: "".to_string(),
                column: field.name().to_string(),
                coltype: column_type,
                colflags: ColumnFlags::empty(),
            })
        }

        fn convert_schema(schema: &DataSchemaRef) -> Result<Vec<Column>> {
            schema.fields().iter().map(make_column_from_field).collect()
        }

        let tz = format.timezone;
        match convert_schema(&query_result.schema) {
            Err(error) => Self::err(&error, dataset_writer).await,
            Ok(columns) => {
                let mut row_writer = dataset_writer.start(&columns).await?;

                let blocks = &mut query_result.blocks;
                while let Some(block) = blocks.next().await {
                    let block = match block {
                        Err(e) => {
                            row_writer
                                .finish_error(
                                    ErrorKind::ER_UNKNOWN_ERROR,
                                    &e.to_string().as_bytes(),
                                )
                                .await?;
                            return Ok(());
                        }
                        Ok(block) => block,
                    };
                    match block.get_serializers() {
                        Ok(serializers) => {
                            let rows_size = block.column(0).len();
                            for row_index in 0..rows_size {
                                for (col_index, serializer) in serializers.iter().enumerate() {
                                    let val = block.column(col_index).get_checked(row_index)?;
                                    if val.is_null() {
                                        row_writer.write_col(None::<u8>)?;
                                        continue;
                                    }
                                    let data_type = remove_nullable(
                                        block.schema().fields()[col_index].data_type(),
                                    );

                                    match (data_type.data_type_id(), val.clone()) {
                                        (TypeID::Boolean, DataValue::Boolean(v)) => {
                                            row_writer.write_col(v as i8)?
                                        }
                                        (TypeID::Date, DataValue::Int64(v)) => {
                                            let v = v as i32;
                                            row_writer.write_col(v.to_date(&tz).naive_local())?
                                        }
                                        (TypeID::Timestamp, DataValue::Int64(_)) => row_writer
                                            .write_col(
                                                serializer.serialize_field(row_index, format)?,
                                            )?,
                                        (TypeID::String, DataValue::String(v)) => {
                                            row_writer.write_col(v)?
                                        }
                                        (TypeID::Array, DataValue::Array(_)) => row_writer
                                            .write_col(
                                                serializer.serialize_field(row_index, format)?,
                                            )?,
                                        (TypeID::Struct, DataValue::Struct(_)) => row_writer
                                            .write_col(
                                                serializer.serialize_field(row_index, format)?,
                                            )?,
                                        (TypeID::Variant, DataValue::Variant(_)) => row_writer
                                            .write_col(
                                                serializer.serialize_field(row_index, format)?,
                                            )?,
                                        (TypeID::VariantArray, DataValue::Variant(_)) => row_writer
                                            .write_col(
                                                serializer.serialize_field(row_index, format)?,
                                            )?,
                                        (TypeID::VariantObject, DataValue::Variant(_)) => {
                                            row_writer.write_col(
                                                serializer.serialize_field(row_index, format)?,
                                            )?
                                        }
                                        (_, DataValue::Int64(v)) => row_writer.write_col(v)?,

                                        (_, DataValue::UInt64(v)) => row_writer.write_col(v)?,

                                        (_, DataValue::Float64(_)) => row_writer
                                            // mysql writer use a text protocol,
                                            // it use format!() to serialize number,
                                            // the result will be different with our serializer for floats
                                            .write_col(
                                                serializer.serialize_field(row_index, format)?,
                                            )?,
                                        (_, v) => {
                                            return Err(ErrorCode::BadDataValueType(format!(
                                                "Unsupported column type:{:?}, expected type in schema: {:?}",
                                                v.data_type(),
                                                data_type
                                            )));
                                        }
                                    }
                                }
                                row_writer.end_row().await?;
                            }
                        }
                        Err(e) => {
                            row_writer
                                .finish_error(
                                    ErrorKind::ER_UNKNOWN_ERROR,
                                    &e.to_string().as_bytes(),
                                )
                                .await?;
                            return Ok(());
                        }
                    }
                }

                row_writer.finish_with_info(&default_response.info).await?;

                Ok(())
            }
        }
    }

    async fn err(error: &ErrorCode, writer: QueryResultWriter<'a, W>) -> Result<()> {
        if error.code() != ABORT_QUERY && error.code() != ABORT_SESSION {
            error!("OnQuery Error: {:?}", error);
            writer
                .error(ErrorKind::ER_UNKNOWN_ERROR, error.to_string().as_bytes())
                .await?;
        } else {
            writer
                .error(
                    ErrorKind::ER_ABORTING_CONNECTION,
                    error.to_string().as_bytes(),
                )
                .await?;
        }

        Ok(())
    }
}
