#!/bin/bash
set -eu

IMAGE=shardlabs/starknet-devnet

function test_and_push(){
    local tagged_image="$IMAGE:$1"

    echo "Run $tagged_image in background; sleep to allow it to start"
    local container_name="devnet"
    docker run -d -p 127.0.0.1:5050:5050 --name "$container_name" --rm "$tagged_image"
    sleep 10 # alternatively check in a loop
    docker logs "$container_name"

    echo "Checking if devnet instance is alive"
    if [ ! -z $REMOTE ]; then
        ssh remote-docker curl localhost:5050/is_alive -w "\n"
    else
        curl localhost:5050/is_alive -w "\n"
    fi

    docker kill "$container_name"
}

SHA1_TAG="${CIRCLE_SHA1}${ARCH_SUFFIX}"
echo "Building regular image: $SHA1_TAG"
docker build . -t "$IMAGE:$SHA1_TAG"

SEED_SUFFIX="-seed0"
SHA1_SEEDED_TAG="${SHA1_TAG}${SEED_SUFFIX}"
echo "Building seeded image: $SHA1_SEEDED_TAG"
docker build . \
    -f seed0.Dockerfile \
    --build-arg BASE_TAG=$SHA1_TAG \
    -t "$IMAGE:$SHA1_SEEDED_TAG"

echo "Images built successfully; proceeding to testing and pushing"
docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"
for pushable_tag in $SHA1_TAG $SHA1_SEEDED_TAG; do
    test_and_push $pushable_tag
done
