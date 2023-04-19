FROM python:3.9.13-alpine3.16 as builder

COPY . .

RUN apk add gmp-dev g++ gcc libffi-dev

RUN ./scripts/install_poetry.sh

# install rustc+cargo
RUN wget https://sh.rustup.rs -O - | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN poetry build -f wheel
RUN poetry export -f requirements.txt --without-hashes --with vm > requirements.txt
RUN pip3 wheel --no-cache-dir --no-deps --wheel-dir /wheels -r requirements.txt

FROM python:3.9.13-alpine3.16

RUN apk add --no-cache libgmpxx

COPY --from=builder /dist/*.whl /wheels/
COPY --from=builder /wheels /wheels

RUN pip3 install --no-cache /wheels/*
RUN rm -rf /wheels

ENV PYTHONUNBUFFERED=1

ENTRYPOINT [ "starknet-devnet", "--host", "0.0.0.0", "--port", "5050" ]
