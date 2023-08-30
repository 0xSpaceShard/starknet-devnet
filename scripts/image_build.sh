#!/bin/bash

# Aside from these sha1-tagged images being useful per-se, they also allow the separation of image building and pushing.
# Image building can thus be run in parallel with testing, and pushing of official/latest versions can be done if testing and building was successful.
# sha1-tagged images are pushed regardless of the tests run in parallel.

set -eu

IMAGE=shardlabs/starknet-devnet-rs

function test_image() {
    local tagged_image="$IMAGE:$1"

    local container_name="devnet"

    local internal_port="5050"
    local external_address="127.0.0.1:5050"

    echo "Runing $tagged_image in background; sleeping to allow it to start"
    # not using --rm so that later logs can be printed if debugging is needed
    docker run -d \
        -p "$external_address:$internal_port" \
        --name "$container_name" \
        "$tagged_image" \
        --port "$internal_port"

    sleep 10 # alternatively check in a loop

    # logging can be helpful if Devnet exited early
    docker logs "$container_name"

    echo "Checking if devnet instance is alive"
    ssh remote-docker curl "$external_address/is_alive" -w "\n"

    docker rm -f "$container_name"
}

# Setup buildx
docker --version
docker run --rm --privileged tonistiigi/binfmt --install arm64
docker buildx ls

docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"

echo "Pushing images tagged with sha1 commit digest"
echo "Temporarily pushing tag latest. Once semver is established for this project, this should be done conditionally in a separate script, as was done with devnet-py"

SHA1_TAG="${CIRCLE_SHA1}"
echo "Building regular image: $SHA1_TAG"
docker buildx build . \
    -t "$IMAGE:$SHA1_TAG" \
    -t "$IMAGE:latest" \
    --platform="$BUILDX_PLATFORMS" \
    --push

SEED_SUFFIX="-seed0"
SHA1_SEEDED_TAG="${SHA1_TAG}${SEED_SUFFIX}"
echo "Building seeded image: $SHA1_SEEDED_TAG"
docker buildx build . \
    -f seed0.Dockerfile \
    --build-arg BASE_TAG=$SHA1_TAG \
    -t "$IMAGE:$SHA1_SEEDED_TAG" \
    --platform="$BUILDX_PLATFORMS" \
    --push

echo "Images built and pushed. Validating."
for testable_tag in $SHA1_TAG $SHA1_SEEDED_TAG; do
    test_image $testable_tag
done
