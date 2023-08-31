#!/bin/bash

# Aside from these sha1-tagged images being useful per-se, they also allow the separation of image building and pushing.
# Image building can thus be run in parallel with testing, and pushing of official/latest versions can be done if testing and building was successful.
# sha1-tagged images are pushed regardless of the tests run in parallel.

set -eu

IMAGE=shardlabs/starknet-devnet-rs

function validate_and_push() {
    local tagged_image="$IMAGE:$1"

    local container_name="devnet"

    local internal_port="5050"
    local external_address="127.0.0.1:5050"

    echo "Running $tagged_image in background; sleeping to allow it to start"
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
    if [ ! -z $REMOTE ]; then
        ssh remote-docker curl "$external_address/is_alive" -w "\n"
    else
        curl "$external_address/is_alive" -w "\n"
    fi

    docker push "$tagged_image"

    docker rm -f "$container_name"
}

echo "Building ${ARCH_SUFFIX} images tagged with sha1 commit digest"

SHA1_TAG="${CIRCLE_SHA1}${ARCH_SUFFIX}"
echo "Building regular (unseeded) image: $SHA1_TAG"
docker build . \
    -f docker/Dockerfile \
    -t "$IMAGE:$SHA1_TAG"

SEED_SUFFIX="-seed0"
SHA1_SEEDED_TAG="${SHA1_TAG}${SEED_SUFFIX}"
echo "Building seeded image: $SHA1_SEEDED_TAG"
docker build . \
    -f docker/seed0.Dockerfile \
    -t "$IMAGE:$SHA1_SEEDED_TAG" \
    --build-arg BASE_TAG=$SHA1_TAG

echo "Images built. Validating and pushing."
docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"

for image_tag in $SHA1_TAG $SHA1_SEEDED_TAG; do
    validate_and_push $image_tag
done
