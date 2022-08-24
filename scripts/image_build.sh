#!/bin/bash
set -eu

IMAGE=shardlabs/starknet-devnet

function test_and_push(){
    local tagged_image="$IMAGE:$1"

    echo "Run a devnet instance in background; sleep to allow it to start"
    docker run -d -p 127.0.0.1:5050:5050 --name devnet "$tagged_image"
    sleep 10
    docker logs devnet

    echo "Checking if devnet instance is alive"
    if [ ! -z $REMOTE ]; then
        ssh remote-docker curl localhost:5050/is_alive -w "\n"
    else
        curl localhost:5050/is_alive -w "\n"
    fi
}

# building regular image
echo "Build image regardless of versioning"
SHA1_TAG="${CIRCLE_SHA1}${ARCH_SUFFIX}"
docker build . -t "$IMAGE:$SHA1_TAG"

# building seeded image
SEED_SUFFIX="-seed0"
SHA1_SEEDED_TAG="${SHA1_TAG}${SEED_SUFFIX}"
docker build . \
    -f seed0.Dockerfile \
    --build-arg BASE_TAG=$SHA1_TAG \
    -t "$IMAGE:$SHA1_SEEDED_TAG"

# testing and pushing built images
docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"
for pushable_tag in $SHA1_TAG $SHA1_SEEDED_TAG; do
    test_and_push $pushable_tag
done
