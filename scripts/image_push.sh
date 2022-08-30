#!/bin/bash
set -eu

IMAGE=shardlabs/starknet-devnet
LOCAL_VERSION=$(./scripts/get_version.sh version)
echo "Local version: $LOCAL_VERSION"

# curling the url fails with 404
function docker_image_exists() {
    curl --silent -f -lSL "$1" > /dev/null 2>&1
}

function tag_and_push() {
    local source_tag="$1"
    local target_tag="$2"

    local source_image="$IMAGE:$source_tag"
    local target_image="$IMAGE:$target_tag"

    docker tag "$source_image" "$target_image"
    docker push "$target_image"
}

DOCKERHUB_URL="https://hub.docker.com/v2/repositories/$IMAGE/tags/$LOCAL_VERSION"
docker_image_exists "$DOCKERHUB_URL" \
    && echo "Image with tag $LOCAL_VERSION already pushed. Skipping!" \
    && exit 0

docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"

for arch_suffix in "" "-arm"; do
    # relies on image_build.sh to have pushed image with SHA1_TAG
    SHA1_TAG="${CIRCLE_SHA1}${arch_suffix}"
    docker pull "$IMAGE:$SHA1_TAG"

    LOCAL_VERSION_TAG="${LOCAL_VERSION}${arch_suffix}"
    LATEST_VERSION_TAG="latest$arch_suffix"

    for target_tag in $LOCAL_VERSION_TAG $LATEST_VERSION_TAG; do
        tag_and_push $SHA1_TAG $target_tag 
    done

    # relies on image_build.sh to have pushed image with SHA1_SEEDED_TAG
    SEED_SUFFIX="-seed0"
    SHA1_SEEDED_TAG="${SHA1_TAG}${SEED_SUFFIX}"
    docker pull "$IMAGE:$SHA1_SEEDED_TAG"

    LOCAL_VERSION_SEEDED_TAG="${LOCAL_VERSION_TAG}${SEED_SUFFIX}"
    LATEST_VERSION_SEEDED_TAG="${LATEST_VERSION_TAG}${SEED_SUFFIX}"

    for target_tag in $LOCAL_VERSION_SEEDED_TAG $LATEST_VERSION_SEEDED_TAG; do
        tag_and_push $SHA1_SEEDED_TAG $target_tag
    done
done
