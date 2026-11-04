FROM docker.io/library/golang:1.24.8-bookworm@sha256:39e2f344b68e61144a5de3d3bd166ea746ee8e1965e6fc2ad046df5cd6c8369e AS server
ENV GOTOOLCHAIN=local CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
    GOFLAGS=-mod=readonly GOPROXY=https://proxy.golang.org GOSUMDB=sum.golang.org \
    GOPRIVATE="" GONOPROXY="" GONOSUMDB="" GOMAXPROCS=2
WORKDIR /src
RUN git init && git remote add origin https://github.com/minio/minio.git \
    && git fetch --depth=1 origin 9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a \
    && git checkout --detach FETCH_HEAD \
    && test "$(git rev-parse HEAD)" = 9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a
RUN go version && go mod download && go mod verify
RUN mkdir /out && flags=$(MINIO_RELEASE=DEVELOPMENT go run buildscripts/gen-ldflags.go) \
    && go build -p 2 -tags kqueue -trimpath -ldflags "$flags" -o /out/minio . \
    && git diff --exit-code -- go.mod go.sum \
    && /out/minio --version > /out/minio-version.txt

FROM server AS client
COPY bucket.go /src/ontoforge-bucket/main.go
RUN go build -p 2 -trimpath -o /out/bucket ./ontoforge-bucket \
    && git diff --exit-code -- go.mod go.sum \
    && /out/bucket version > /out/bucket-version.txt \
    && cd /out && sha256sum minio bucket > SHA256SUMS

FROM scratch
COPY --from=client /out/ /
COPY --from=client /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
LABEL org.opencontainers.image.source="https://github.com/minio/minio" \
      org.opencontainers.image.revision="9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a"
ENTRYPOINT ["/minio"]
