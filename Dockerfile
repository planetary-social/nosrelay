FROM ubuntu:jammy as build

WORKDIR /build
RUN apt update && apt install -y --no-install-recommends \
    git g++ make pkg-config libtool ca-certificates \
    libyaml-perl libtemplate-perl libregexp-grammars-perl libssl-dev zlib1g-dev \
    liblmdb-dev libflatbuffers-dev libsecp256k1-dev \
    libzstd-dev

RUN git clone --branch 0.9.6 https://github.com/hoytech/strfry.git && cd strfry/ \
    && git submodule update --init \
    && make setup-golpe \
    && make clean \
    && make -j4

FROM ubuntu:jammy as runner

EXPOSE 7777

RUN curl -fsSL https://deno.land/install.sh | sh
RUN apt-get update && apt-get install -y --no-install-recommends  \
    vim \
    liblmdb0 libflatbuffers1 libsecp256k1-0 libb2-1 libzstd1 \
    && rm -rf /var/lib/apt/lists/*

COPY ./strfry/config/strfry.conf /etc/strfry.conf

RUN mkdir -p /app/strfry-db
COPY ./strfry/plugins/ /app/plugins/

RUN chmod +x /app/plugins/allowed_rules.js

WORKDIR /app
COPY --from=build /build/strfry/strfry strfry

ENTRYPOINT ["/app/strfry", "relay"]