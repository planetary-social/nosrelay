#!/bin/bash
# Pull specified image sha and tag it as news to trigger the deploy process

SOURCE=$1
if [ -z "$SOURCE" ]; then
  echo "Usage: $0 <image_sha>"
  exit 1
fi

docker pull --platform linux/amd64 ghcr.io/planetary-social/nosrelay:$SOURCE && \
docker tag ghcr.io/planetary-social/nosrelay:$SOURCE ghcr.io/planetary-social/nosrelay:news && \
docker push ghcr.io/planetary-social/nosrelay:news