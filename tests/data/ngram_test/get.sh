for year in {2010..2015}; do
    wget https://datasets-documentation.s3.eu-west-3.amazonaws.com/amazon_reviews/amazon_reviews_${year}.snappy.parquet
done