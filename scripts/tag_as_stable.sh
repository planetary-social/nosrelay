#!/bin/bash
# Pull specified image sha and tag it as stable to trigger the deploy process

SOURCE=$1
if [ -z "$SOURCE" ]; then
  echo "Usage: $0 <image_sha>"
  exit 1
fi

docker pull --platform linux/amd64 ghcr.io/planetary-social/nosrelay@sha256:$SOURCE && \
docker tag ghcr.io/planetary-social/nosrelay@sha256:$SOURCE ghcr.io/planetary-social/nosrelay:stable && \
docker push ghcr.io/planetary-social/nosrelay:stable