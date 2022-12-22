FROM python:3.9.13-alpine3.16 as builder

COPY . .

RUN apk add gmp-dev g++ gcc git libffi-dev

RUN pip3 install poetry

RUN poetry build -f wheel
RUN poetry export -f requirements.txt --without-hashes > requirements.txt
RUN pip3 wheel --no-cache-dir --no-deps --wheel-dir /wheels -r requirements.txt

# install rust
RUN wget https://sh.rustup.rs -O - | sh -s -- -y 
RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

# build-cairo-rs-py wheel with maturin
RUN git clone https://github.com/lambdaclass/cairo-rs-py.git
RUN pip3 install maturin[patchelf]
RUN maturin build -m cairo-rs-py/Cargo.toml --no-default-features --features extension

FROM python:3.9.13-alpine3.16

RUN apk add --no-cache libgmpxx

COPY --from=builder /cairo-rs-py/target/wheels/*.whl /wheels/
COPY --from=builder /dist/*.whl /wheels/
COPY --from=builder /wheels /wheels

RUN pip3 install --no-cache /wheels/*
RUN rm -rf /wheels

ENV PYTHONUNBUFFERED=1

ENTRYPOINT [ "starknet-devnet", "--host", "0.0.0.0", "--port", "5050" ]
