FROM python:3.12-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1 \
    HF_HOME=/opt/huggingface \
    TRANSFORMERS_CACHE=/opt/huggingface

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Install CPU-only torch to avoid downloading CUDA packages during sandbox startup.
RUN python -m pip install --upgrade pip setuptools wheel && \
    python -m pip install --index-url https://download.pytorch.org/whl/cpu torch && \
    python -m pip install databend-udf transformers

# Warm up the default sentiment model so first UDF call can start faster.
RUN python - <<'PY'
from transformers import pipeline
pipeline("sentiment-analysis")
print("Preloaded sentiment-analysis model")
PY

WORKDIR /app
