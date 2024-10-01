FROM ubuntu:jammy AS build

WORKDIR /build

RUN apt update && apt install -y --no-install-recommends \
    git g++ make pkg-config libtool ca-certificates \
    libyaml-perl libtemplate-perl libregexp-grammars-perl libssl-dev zlib1g-dev \
    liblmdb-dev libflatbuffers-dev libsecp256k1-dev libzstd-dev curl build-essential

RUN git clone --branch 0.9.6 https://github.com/hoytech/strfry.git && \
    cd strfry/ && \
    git submodule update --init && \
    make setup-golpe && \
    make clean && \
    make -j4

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain 1.80.1
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustc --version

COPY ./event_deleter/Cargo.toml ./event_deleter/Cargo.lock /build/event_deleter/

WORKDIR /build/event_deleter

RUN cargo fetch

COPY ./event_deleter/src /build/event_deleter/src

RUN cargo build --release

FROM ubuntu:jammy AS runner

EXPOSE 7777

RUN apt-get update && apt-get install -y --no-install-recommends  \
    vim curl unzip ca-certificates \
    liblmdb0 libflatbuffers1 libsecp256k1-0 libb2-1 libzstd1 \
    && rm -rf /var/lib/apt/lists/*

RUN update-ca-certificates

RUN curl -fsSL https://deno.land/install.sh | sh
ENV DENO_INSTALL="/root/.deno"
ENV PATH="$DENO_INSTALL/bin:$PATH"
RUN deno --version

COPY ./strfry/config/strfry.conf /etc/strfry.conf
RUN mkdir -p /app/strfry-db
COPY ./strfry/plugins/ /app/plugins/
RUN chmod +x /app/plugins/policies.ts

WORKDIR /app

COPY --from=build /build/strfry/strfry strfry

COPY --from=build /build/event_deleter/target/release/spam_cleaner /usr/local/bin/spam_cleaner
COPY --from=build /build/event_deleter/target/release/vanish_subscriber vanish_subscriber

RUN chmod +x /usr/local/bin/spam_cleaner
RUN chmod +x /app/vanish_subscriber

COPY ./start.sh start.sh
CMD ./start.sh
