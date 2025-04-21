CREATE OR REPLACE TABLE t2(a int not null, b int not null, c varchar) bloom_index_columns='a,b' ngram_index_columns='c';

INSERT INTO TABLE t2 values(1,2,'hello'),(4,5,'world');

select * from t2 where c like '%llo%';

create stage data url='fs:///home/kould/RustroverProjects/databend/tests/data/';

CREATE OR REPLACE TABLE `amazon_reviews_bloom` (
                                  `review_date` int(11) NULL,
                                  `marketplace` varchar(20) NULL,
                                  `customer_id` bigint(20) NULL,
                                  `review_id` varchar(40) NULL,
                                  `product_id` varchar(10) NULL,
                                  `product_parent` bigint(20) NULL,
                                  `product_title` varchar(500) NULL,
                                  `product_category` varchar(50) NULL,
                                  `star_rating` smallint(6) NULL,
                                  `helpful_votes` int(11) NULL,
                                  `total_votes` int(11) NULL,
                                  `vine` boolean NULL,
                                  `verified_purchase` boolean NULL,
                                  `review_headline` varchar(500) NULL,
                                  `review_body` string NULL
) Engine = Fuse bloom_index_columns='total_votes';


copy into amazon_reviews_bloom from @data/ngram_test/amazon_reviews_2010.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_bloom from @data/ngram_test/amazon_reviews_2011.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_bloom from @data/ngram_test/amazon_reviews_2012.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_bloom from @data/ngram_test/amazon_reviews_2013.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_bloom from @data/ngram_test/amazon_reviews_2014.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_bloom from @data/ngram_test/amazon_reviews_2015.snappy.parquet file_format = (type = PARQUET);

CREATE OR REPLACE TABLE `amazon_reviews_ngram` (
                                  `review_date` int(11) NULL,
                                  `marketplace` varchar(20) NULL,
                                  `customer_id` bigint(20) NULL,
                                  `review_id` varchar(40) NULL,
                                  `product_id` varchar(10) NULL,
                                  `product_parent` bigint(20) NULL,
                                  `product_title` varchar(500) NULL,
                                  `product_category` varchar(50) NULL,
                                  `star_rating` smallint(6) NULL,
                                  `helpful_votes` int(11) NULL,
                                  `total_votes` int(11) NULL,
                                  `vine` boolean NULL,
                                  `verified_purchase` boolean NULL,
                                  `review_headline` varchar(500) NULL,
                                  `review_body` string NULL
) Engine = Fuse bloom_index_columns='' ngram_index_columns='review_body';

copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2010.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2011.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2012.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2013.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2014.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2015.snappy.parquet file_format = (type = PARQUET);

SELECT
    product_id
FROM
    amazon_reviews
WHERE
    review_body LIKE '%is super awesome%'
ORDER BY
    product_id
    LIMIT 5;

create stage data url='fs:///home/kould/RustroverProjects/databend/tests/data/';

SELECT
    product_id,
    any(product_title),
    AVG(star_rating) AS rating,
    COUNT() AS count
FROM
    amazon_reviews_bloom
WHERE
    review_body = 'is super'
GROUP BY
    product_id
ORDER BY
    count DESC,
    rating DESC,
    product_id
    LIMIT 5;

copy into @data/review_body from (select ngram(review_body, 10) from amazon_reviews_ngram limit 1_000_000 ) file_format=(type=csv) MAX_FILE_SIZE = 18446744073709551615;

SELECT
    product_id,
    any(product_title),
    AVG(star_rating) AS rating,
    COUNT() AS count
FROM
    amazon_reviews_bloom
WHERE
    total_votes = 555
GROUP BY
    product_id
ORDER BY
    count DESC,
    rating DESC,
    product_id
    LIMIT 5;


SELECT
    product_id,
    any(product_title),
    AVG(star_rating) AS rating,
    COUNT() AS count
FROM
    amazon_reviews
WHERE
    review_body LIKE '%Lots of power available. It will run my freezer, frig%'
GROUP BY
    product_id
ORDER BY
    count DESC,
    rating DESC,
    product_id
    LIMIT 5;
