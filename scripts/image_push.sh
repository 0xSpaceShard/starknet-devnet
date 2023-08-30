#!/bin/bash
set -eu

IMAGE=shardlabs/starknet-devnet-rs

docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"

for image_suffix in arm amd arm-seed0 amd-seed0; do
    docker pull "$IMAGE:$image_suffix"
done

for seed_suffix in "" "-seed0"; do
    for image_tag in "$CIRCLE_SHA1" "latest"; do
        joint_manifest="$IMAGE:${image_tag}${seed_suffix}"
        docker manifest create $joint_manifest \
            "$IMAGE:${image_tag}-arm${seed_suffix}" \
            "$IMAGE:${image_tag}-amd${seed_suffix}"

        docker manifest push $joint_manifest
    done
done

# TODO
echo "Temporarily pushing tag latest. Once semver is established for this project, this should be done conditionally in a separate script, as was done with devnet-py"
