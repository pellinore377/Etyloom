FROM python:3.12-slim-bookworm
ARG TORCH_INDEX=https://download.pytorch.org/whl/cpu
RUN pip install --no-cache-dir torch==2.10.0 --index-url ${TORCH_INDEX} \
    && groupadd --gid 10001 etyloom \
    && useradd --uid 10001 --gid etyloom --no-create-home etyloom \
    && mkdir -p /models && chown etyloom:etyloom /models
WORKDIR /app
COPY train.py infer.py test_neural.py ./
USER 10001:10001
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1 OMP_NUM_THREADS=4
ENTRYPOINT ["python", "train.py"]
