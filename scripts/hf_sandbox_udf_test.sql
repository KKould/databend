-- Prerequisite:
-- 1) query.common.enable_udf_sandbox = true
-- 2) sandbox worker has outbound network access (for pip/model download)

DROP FUNCTION IF EXISTS hf_sentiment_score;

-- 提供一个预构建的torch、hugging face相关依赖的镜像更好
CREATE OR REPLACE FUNCTION hf_sentiment_score (STRING)
  RETURNS STRING
  LANGUAGE python
  IMPORTS = ()
  PACKAGES = ()
  HANDLER = 'hf_sentiment_score'
  AS $$
  from transformers import pipeline
  _classifier = None

  def _get_classifier():
      global _classifier
      if _classifier is None:
          _classifier = pipeline("sentiment-analysis")
      return _classifier

  def hf_sentiment_score(text):
      result = _get_classifier()(text)[0]
      label = str(result["label"])
      score = str(float(result["score"]))
      return label + ':' + score
  $$;

-- If you are not using a prebuilt runtime image, first call can still be slow.
SELECT hf_sentiment_score('I love Databend!') AS sentiment;
SELECT hf_sentiment_score('We hope you don''t hate it.') AS sentiment;
