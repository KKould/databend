// Copyright 2021 Datafuse Labs
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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use databend_common_expression::DataBlock;
use databend_common_expression::DataSchema;
use databend_common_expression::DataSchemaRef;
use databend_common_expression::TableDataType;
use databend_common_expression::TableField;
use databend_common_expression::TableSchemaRef;
use databend_common_expression::TableSchemaRefExt;
use databend_common_expression::types::StringType;
use databend_common_expression::utils::FromData;
use regex::Regex;

type LazyBlockFunc = fn(&str) -> Option<(TableSchemaRef, DataBlock)>;

struct FederatedHelper;

impl FederatedHelper {
    fn block_match_rule(
        query: &str,
        rules: &[(Regex, Option<(TableSchemaRef, DataBlock)>)],
    ) -> Option<(TableSchemaRef, DataBlock)> {
        for (regex, data) in rules.iter() {
            if regex.is_match(query) {
                return match data {
                    None => Some((TableSchemaRefExt::create(vec![]), DataBlock::empty())),
                    Some((schema, data_block)) => Some((schema.clone(), data_block.clone())),
                };
            }
        }

        None
    }

    fn lazy_block_match_rule(
        query: &str,
        rules: &[(Regex, LazyBlockFunc)],
    ) -> Option<(TableSchemaRef, DataBlock)> {
        for (regex, func) in rules.iter() {
            if regex.is_match(query) {
                return match func(query) {
                    None => Some((TableSchemaRefExt::create(vec![]), DataBlock::empty())),
                    Some((schema, data_block)) => Some((schema, data_block)),
                };
            }
        }
        None
    }
}

pub struct MySQLFederated;

impl MySQLFederated {
    pub fn create() -> Self {
        MySQLFederated
    }

    // Build block for select function.
    // Format:
    // |function_name|
    // |value|
    fn select_function_block(name: &str, value: &str) -> Option<(TableSchemaRef, DataBlock)> {
        let schema = TableSchemaRefExt::create(vec![TableField::new(name, TableDataType::String)]);
        let block =
            DataBlock::new_from_columns(vec![StringType::from_data(vec![value.to_string()])]);
        Some((schema, block))
    }

    // Build block for show variable statement.
    // Format is:
    // |variable_name| Value|
    // | xx          | yy   |
    fn show_variables_block(name: &str, value: &str) -> Option<(TableSchemaRef, DataBlock)> {
        let schema = TableSchemaRefExt::create(vec![
            TableField::new("Variable_name", TableDataType::String),
            TableField::new("Value", TableDataType::String),
        ]);
        let block = DataBlock::new_from_columns(vec![
            StringType::from_data(vec![name.to_string()]),
            StringType::from_data(vec![value.to_string()]),
        ]);
        Some((schema, block))
    }

    // Returns empty result set with MySQL-compatible schema for SHOW KEYS/INDEX commands.
    // Required for PowerBI + MySQL ODBC 9.5 compatibility.
    // Reference: https://dev.mysql.com/doc/refman/8.0/en/show-index.html
    fn show_keys_block() -> Option<(TableSchemaRef, DataBlock)> {
        use databend_common_expression::types::number::*;

        let schema = TableSchemaRefExt::create(vec![
            TableField::new("Table", TableDataType::String),
            TableField::new(
                "Non_unique",
                TableDataType::Number(databend_common_expression::types::NumberDataType::Int32),
            ),
            TableField::new("Key_name", TableDataType::String),
            TableField::new(
                "Seq_in_index",
                TableDataType::Number(databend_common_expression::types::NumberDataType::Int32),
            ),
            TableField::new("Column_name", TableDataType::String),
            TableField::new(
                "Collation",
                TableDataType::Nullable(Box::new(TableDataType::String)),
            ),
            TableField::new(
                "Cardinality",
                TableDataType::Nullable(Box::new(TableDataType::Number(
                    databend_common_expression::types::NumberDataType::Int64,
                ))),
            ),
            TableField::new(
                "Sub_part",
                TableDataType::Nullable(Box::new(TableDataType::Number(
                    databend_common_expression::types::NumberDataType::Int64,
                ))),
            ),
            TableField::new(
                "Packed",
                TableDataType::Nullable(Box::new(TableDataType::String)),
            ),
            TableField::new("Null", TableDataType::String),
            TableField::new("Index_type", TableDataType::String),
            TableField::new("Comment", TableDataType::String),
            TableField::new("Index_comment", TableDataType::String),
            TableField::new("Visible", TableDataType::String),
            TableField::new(
                "Expression",
                TableDataType::Nullable(Box::new(TableDataType::String)),
            ),
        ]);

        let block = DataBlock::new_from_columns(vec![
            StringType::from_data(Vec::<String>::new()),
            Int32Type::from_data(Vec::<i32>::new()),
            StringType::from_data(Vec::<String>::new()),
            Int32Type::from_data(Vec::<i32>::new()),
            StringType::from_data(Vec::<String>::new()),
            StringType::from_data_with_validity(Vec::<&str>::new(), Vec::<bool>::new()),
            Int64Type::from_data_with_validity(Vec::<i64>::new(), Vec::<bool>::new()),
            Int64Type::from_data_with_validity(Vec::<i64>::new(), Vec::<bool>::new()),
            StringType::from_data_with_validity(Vec::<&str>::new(), Vec::<bool>::new()),
            StringType::from_data(Vec::<String>::new()),
            StringType::from_data(Vec::<String>::new()),
            StringType::from_data(Vec::<String>::new()),
            StringType::from_data(Vec::<String>::new()),
            StringType::from_data(Vec::<String>::new()),
            StringType::from_data_with_validity(Vec::<&str>::new(), Vec::<bool>::new()),
        ]);

        Some((schema, block))
    }

    fn select_variable_data_block(query: &str) -> Option<(TableSchemaRef, DataBlock)> {
        let mut default_map = HashMap::new();
        default_map.insert("tx_isolation", "REPEATABLE-READ");
        default_map.insert("session.tx_isolation", "REPEATABLE-READ");
        default_map.insert("transaction_isolation", "REPEATABLE-READ");
        default_map.insert("session.transaction_isolation", "REPEATABLE-READ");
        default_map.insert("session.transaction_read_only", "0");
        default_map.insert("time_zone", "UTC");
        default_map.insert("system_time_zone", "UTC");
        default_map.insert("max_allowed_packet", "134217728");
        default_map.insert("interactive_timeout", "31536000");
        default_map.insert("wait_timeout", "31536000");
        default_map.insert("net_write_timeout", "31536000");

        let mut fields = vec![];
        let mut values = vec![];

        let query = query.to_lowercase();
        let mut vars: Vec<&str> = query.split("@@").collect();
        if vars.len() > 1 {
            vars.remove(0);
            for var in vars {
                let var = var.trim_end_matches([' ', ',']);
                let vars_as: Vec<&str> = var.split(" as ").collect();
                if vars_as.len() == 2 {
                    let var_as = vars_as[1];
                    fields.push(TableField::new(var_as, TableDataType::String));

                    let var = vars_as[0];
                    let value = default_map.get(var).unwrap_or(&"0").to_string();
                    values.push(StringType::from_data(vec![value]));
                } else {
                    fields.push(TableField::new(
                        &format!("@@{}", var),
                        TableDataType::String,
                    ));

                    let value = default_map.get(var).unwrap_or(&"0").to_string();
                    values.push(StringType::from_data(vec![value]));
                }
            }
        }

        let schema = TableSchemaRefExt::create(fields);
        let block = DataBlock::new_from_columns(values);
        Some((schema, block))
    }

    fn federated_select_variable_check(&self, query: &str) -> Option<(TableSchemaRef, DataBlock)> {
        static SELECT_VARIABLES_LAZY_RULES: LazyLock<Vec<(Regex, LazyBlockFunc)>> =
            LazyLock::new(|| {
                vec![
                    (
                        Regex::new("(?i)^(SELECT @@(.*))").unwrap(),
                        MySQLFederated::select_variable_data_block,
                    ),
                    (
                        Regex::new("(?i)^(/\\* mysql-connector-java(.*))").unwrap(),
                        MySQLFederated::select_variable_data_block,
                    ),
                ]
            });

        FederatedHelper::lazy_block_match_rule(query, &SELECT_VARIABLES_LAZY_RULES)
    }

    fn federated_show_variables_check(&self, query: &str) -> Option<(TableSchemaRef, DataBlock)> {
        #![allow(clippy::type_complexity)]
        static SHOW_VARIABLES_RULES: LazyLock<Vec<(Regex, Option<(TableSchemaRef, DataBlock)>)>> =
            LazyLock::new(|| {
                vec![
                    (
                        Regex::new("(?i)^(SHOW VARIABLES LIKE 'sql_mode'(.*))").unwrap(),
                        MySQLFederated::show_variables_block(
                            "sql_mode",
                            "ONLY_FULL_GROUP_BY STRICT_TRANS_TABLES NO_ZERO_IN_DATE NO_ZERO_DATE ERROR_FOR_DIVISION_BY_ZERO NO_ENGINE_SUBSTITUTION",
                        ),
                    ),
                    (
                        Regex::new("(?i)^(SHOW VARIABLES LIKE 'lower_case_table_names'(.*))")
                            .unwrap(),
                        MySQLFederated::show_variables_block("lower_case_table_names", "0"),
                    ),
                    (
                        Regex::new("(?i)^(show collation where(.*))").unwrap(),
                        MySQLFederated::show_variables_block("", ""),
                    ),
                    (
                        Regex::new("(?i)^(SHOW VARIABLES(.*))").unwrap(),
                        MySQLFederated::show_variables_block("", ""),
                    ),
                ]
            });

        FederatedHelper::block_match_rule(query, &SHOW_VARIABLES_RULES)
    }

    fn federated_mixed_check(&self, query: &str) -> Option<(TableSchemaRef, DataBlock)> {
        #![allow(clippy::type_complexity)]
        static MIXED_RULES: LazyLock<Vec<(Regex, Option<(TableSchemaRef, DataBlock)>)>> =
            LazyLock::new(|| {
                vec![
                    (Regex::new("(?i)^(START(.*))").unwrap(), None),
                    (Regex::new("(?i)^(SET NAMES(.*))").unwrap(), None),
                    (
                        Regex::new("(?i)^(SET character_set_results(.*))").unwrap(),
                        None,
                    ),
                    (Regex::new("(?i)^(SET net_write_timeout(.*))").unwrap(), None),
                    (
                        Regex::new("(?i)^(SET FOREIGN_KEY_CHECKS(.*))").unwrap(),
                        None,
                    ),
                    (Regex::new("(?i)^(SET AUTOCOMMIT(.*))").unwrap(), None),
                    (Regex::new("(?i)^(SET SQL_LOG_BIN(.*))").unwrap(), None),
                    (Regex::new("(?i)^(SET sql_mode(.*))").unwrap(), None),
                    (Regex::new("(?i)^(SET SQL_SELECT_LIMIT(.*))").unwrap(), None),
                    (Regex::new("(?i)^(SET @@(.*))").unwrap(), None),
                    (Regex::new("(?i)^(SHOW COLLATION)").unwrap(), None),
                    (Regex::new("(?i)^(SHOW CHARSET)").unwrap(), None),
                    (
                        Regex::new("(?i)^(SELECT TIMEDIFF\\(NOW\\(\\), UTC_TIMESTAMP\\(\\)\\))")
                            .unwrap(),
                        MySQLFederated::select_function_block(
                            "TIMEDIFF(NOW(), UTC_TIMESTAMP())",
                            "00:00:00",
                        ),
                    ),
                    (Regex::new("(?i)^(SET SESSION(.*))").unwrap(), None),
                    (
                        Regex::new("(?i)^(SET SQL_QUOTE_SHOW_CREATE(.*))").unwrap(),
                        None,
                    ),
                    (Regex::new("(?i)^(LOCK TABLES(.*))").unwrap(), None),
                    (Regex::new("(?i)^(UNLOCK TABLES(.*))").unwrap(), None),
                    (
                        Regex::new("(?i)^(SELECT LOGFILE_GROUP_NAME, FILE_NAME, TOTAL_EXTENTS, INITIAL_SIZE, ENGINE, EXTRA FROM INFORMATION_SCHEMA.FILES(.*))").unwrap(),
                        None,
                    ),
                    (Regex::new("(?i)^(/\\*!80003 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(SHOW MASTER STATUS)").unwrap(), None),
                    (Regex::new("(?i)^(SHOW ALL SLAVES STATUS)").unwrap(), None),
                    (Regex::new("(?i)^(LOCK BINLOG FOR BACKUP)").unwrap(), None),
                    (Regex::new("(?i)^(LOCK TABLES FOR BACKUP)").unwrap(), None),
                    (Regex::new("(?i)^(UNLOCK BINLOG(.*))").unwrap(), None),
                    (Regex::new("(?i)^(/\\*!40101 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(SET SQL_AUTO_IS_NULL(.*))").unwrap(), None),
                    (
                        Regex::new("(?i)^(SHOW KEYS FROM(.*))").unwrap(),
                        MySQLFederated::show_keys_block(),
                    ),
                    (Regex::new("(?i)^(SHOW WARNINGS)").unwrap(), None),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SHOW WARNINGS)")
                            .unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SHOW PLUGINS)")
                            .unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SHOW COLLATION)")
                            .unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SHOW CHARSET)").unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SHOW ENGINES)").unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SELECT @@(.*))")
                            .unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SHOW @@(.*))").unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SET net_write_timeout(.*))")
                            .unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SET SQL_SELECT_LIMIT(.*))")
                            .unwrap(),
                        None,
                    ),
                    (
                        Regex::new("(?i)^(/\\* ApplicationName=(.*)SHOW VARIABLES(.*))")
                            .unwrap(),
                        None,
                    ),
                    (Regex::new("(?i)^(/\\*!40100 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(/\\*!40103 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(/\\*!40111 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(/\\*!40101 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(/\\*!40014 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(/\\*!40000 SET(.*) \\*/)$").unwrap(), None),
                    (Regex::new("(?i)^(/\\*!40000 ALTER(.*) \\*/)$").unwrap(), None),
                    (
                        Regex::new("(?i)^(SELECT 1 FROM DUAL)$").unwrap(),
                        MySQLFederated::select_function_block("1", "1"),
                    ),
                ]
            });

        FederatedHelper::block_match_rule(query, &MIXED_RULES)
    }

    pub fn check(&self, query: &str) -> Option<(DataSchemaRef, DataBlock)> {
        let select_variable = self
            .federated_select_variable_check(query)
            .map(|(schema, chunk)| (Arc::new(DataSchema::from(schema)), chunk));
        if select_variable.is_some() {
            return select_variable;
        }

        let show_variables = self
            .federated_show_variables_check(query)
            .map(|(schema, chunk)| (Arc::new(DataSchema::from(schema)), chunk));
        if show_variables.is_some() {
            return show_variables;
        }

        self.federated_mixed_check(query)
            .map(|(schema, chunk)| (Arc::new(DataSchema::from(schema)), chunk))
    }
}
