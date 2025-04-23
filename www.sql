CREATE OR REPLACE TABLE t1 (id int, content string, NGRAM INDEX idx1 (content) gram_size = 3);

select name, type, original, definition from system.indexes;

INSERT INTO t1 VALUES (0, 'hello world');
INSERT INTO t1 VALUES (1, 'hell1 world');
INSERT INTO t1 VALUES (2, 'hell2 world');
INSERT INTO t1 VALUES (3, 'hell3 world');
INSERT INTO t1 VALUES (4, 'hell4 world');

INSERT INTO t1 VALUES (5, 'hell5 world');
INSERT INTO t1 VALUES (6, 'hell6 world');
INSERT INTO t1 VALUES (7, 'hell7 world');
INSERT INTO t1 VALUES (8, 'hell8 world');
INSERT INTO t1 VALUES (9, 'hell9 world');

select * from t1 where content like '%hello%';



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
) Engine = Fuse bloom_index_columns='review_body';

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
                                  `review_body` string NULL,
                                  NGRAM INDEX idx1 (review_body) gram_size = 10 bitmap_size = 10240
) Engine = Fuse bloom_index_columns='review_body';

copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2010.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2011.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2012.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2013.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2014.snappy.parquet file_format = (type = PARQUET);
copy into amazon_reviews_ngram from @data/ngram_test/amazon_reviews_2015.snappy.parquet file_format = (type = PARQUET);

SELECT COUNT() FROM amazon_reviews_ngram;




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

copy into @data/review_body from (select ngram(review_body, 10) from amazon_reviews_ngram limit 40_000_000 ) file_format=(type=csv) MAX_FILE_SIZE = 18446744073709551615;

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
    amazon_reviews_ngram
WHERE
    review_body LIKE '%The first track with Chris Botti is beautiful%'
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
    amazon_reviews_ngram
WHERE
    review_body LIKE '%is super awesome%'
GROUP BY
    product_id
ORDER BY
    count DESC,
    rating DESC,
    product_id
    LIMIT 5;
