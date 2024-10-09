# Stage 1: Build
#ARG BUILDPLATFORM=linux/amd64

FROM --platform=$BUILDPLATFORM ubuntu:jammy AS build

ARG TARGETPLATFORM
ARG BUILDPLATFORM

WORKDIR /build

RUN apt-get update && apt-get install -y gnupg
RUN apt-key adv --refresh-keys --keyserver keyserver.ubuntu.com

RUN apt update && apt install -y --no-install-recommends \
    unzip cmake git g++ make pkg-config libtool ca-certificates \
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
COPY ./event_deleter/src /build/event_deleter/src
WORKDIR /build/event_deleter
RUN cargo fetch
RUN cargo build --release --bins --tests

RUN ls /build/event_deleter
RUN ls /build/event_deleter/target
RUN find /build/event_deleter/target -type d
ENV DENO_VERSION=v1.46.3
RUN curl -fsSL https://deno.land/install.sh | sh
ENV DENO_INSTALL="/root/.deno"
ENV PATH="$DENO_INSTALL/bin:$PATH"
RUN echo "Deno is located at: $(which deno)"

RUN curl -L https://github.com/fiatjaf/nak/releases/download/v0.7.6/nak-v0.7.6-linux-amd64 -o /usr/local/bin/nak && \
    chmod +x /usr/local/bin/nak

RUN curl -L https://github.com/IBM-Cloud/redli/releases/download/v0.13.0/redli_0.13.0_linux_amd64.tar.gz -o /tmp/redli.tar.gz && \
    tar -xvf /tmp/redli.tar.gz -C /usr/local/bin/ redli_linux_amd64 && \
    mv /usr/local/bin/redli_linux_amd64 /usr/local/bin/redli && \
    chmod +x /usr/local/bin/redli

RUN nak --version
RUN redli --version

# Stage 2: tests
FROM --platform=${BUILDPLATFORM} ubuntu:jammy AS tests

COPY --from=build /usr/local/bin/nak /usr/local/bin/nak
RUN chmod +x /usr/local/bin/nak

RUN apt update && apt install -y --no-install-recommends \
    curl jq git g++ make pkg-config libtool ca-certificates \
    libyaml-perl libtemplate-perl libregexp-grammars-perl libssl-dev zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

COPY ./push_vanish_request.ts /usr/local/bin/push_vanish_request.ts
COPY ./run_integration_tests.sh /usr/local/bin/run_integration_tests.sh

RUN chmod +x /usr/local/bin/push_vanish_request.ts
RUN chmod +x /usr/local/bin/run_integration_tests.sh

COPY --from=build /build/event_deleter/Cargo.toml /tests/event_deleter/
COPY --from=build /build/event_deleter/Cargo.lock /tests/event_deleter/
COPY --from=build /build/event_deleter/src /tests/event_deleter/src
COPY --from=build /build/event_deleter/target /tests/event_deleter/target

COPY --from=build /root/.cargo /root/.cargo
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup default stable

COPY ./strfry/plugins/ /tests/strfry/plugins/
COPY ./run_tests.sh /usr/local/bin/run_tests.sh
RUN chmod +x /usr/local/bin/run_tests.sh

WORKDIR /tests

COPY --from=build /root/.deno /root/.deno
ENV PATH="/root/.deno/bin:${PATH}"

RUN deno --version

CMD ["run_tests.sh"]

# Stage 3: runner
FROM --platform=${BUILDPLATFORM} ubuntu:jammy AS runner

RUN apt-get update && apt-get install -y --no-install-recommends  \
    vim curl jq ca-certificates \
    liblmdb0 libflatbuffers1 libsecp256k1-0 libb2-1 libzstd1 \
    && rm -rf /var/lib/apt/lists/*

RUN update-ca-certificates

COPY --from=build /root/.deno /root/.deno
ENV PATH="/root/.deno/bin:${PATH}"
RUN deno --version

EXPOSE 7777

COPY ./strfry/config/strfry.conf /etc/strfry.conf
RUN mkdir -p /app/strfry-db
COPY ./strfry/plugins/ /app/plugins/
RUN chmod +x /app/plugins/policies.ts

WORKDIR /app

COPY --from=build /build/strfry/strfry strfry
COPY --from=build /build/event_deleter/target/release/spam_cleaner /usr/local/bin/spam_cleaner
COPY --from=build /build/event_deleter/target/release/vanish_subscriber ./vanish_subscriber
COPY --from=build /usr/local/bin/nak /usr/local/bin/nak
COPY --from=build /usr/local/bin/redli /usr/local/bin/redli
COPY ./push_vanish_request.ts /app/push_vanish_request.ts

RUN chmod +x /app/vanish_subscriber

# Tools
RUN chmod +x /usr/local/bin/nak
RUN chmod +x /usr/local/bin/redli
RUN chmod +x /usr/local/bin/spam_cleaner
RUN chmod +x /app/push_vanish_request.ts

COPY ./start.sh start.sh
CMD ./start.sh
