#!/bin/bash
set -u

IMAGE=shardlabs/starknet-devnet-rs

cargo install cargo-get

echo "Logging in to docker hub";
docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"

# Get the version of the binary crate
workspace_members=$(cargo get --delimiter ";" workspace.members)
current_dir="$(pwd)"

IFS=';' read -a array <<< "$workspace_members"

bin_crate_version="";

for workspace_member in "${array[@]}" 
do
    crate_dir="$current_dir/$workspace_member";

    # Check if the Cargo.toml file contains a [[bin]] section
    if grep -q '\[\[bin\]\]' "$crate_dir/Cargo.toml"; then
        echo "The Cargo.toml file in $crate_dir contains a [[bin]] section"
        bin_crate_version=$(cargo get --entry $crate_dir package.version)
        break
    fi
done

function image_exists() {
    docker manifest inspect $1 > /dev/null 2>&1
}

function create_and_push_manifest() {
    local image=$1;
    local manifest_prefix=$2;
    local seed_suffix="";

    if [ -n "${3:-}" ]; then
        seed_suffix=$3
    fi

    local joint_manifest="$image:${manifest_prefix}${seed_suffix}"

    docker manifest create $joint_manifest \
        "$image:${CIRCLE_SHA1}-arm${seed_suffix}" \
        "$image:${CIRCLE_SHA1}-amd${seed_suffix}"

    docker manifest push $joint_manifest
}

echo "Creating a joint docker manifest for each pair of -arm and -amd images."

# construct the image tag from the version
# check if the image tag exists in docker registry
# If it does, do not publish the version
for seed_suffix in "" "-seed0"; do
    # Pull the pair
    # and tag the image with the crate version if not done yet
    for image_suffix in "-arm" "-amd"; do
        image_tag_with_commit_hash="$IMAGE:${CIRCLE_SHA1}${image_suffix}${seed_suffix}"
        docker pull $image_tag_with_commit_hash

        image_tag_with_version="$IMAGE:${bin_crate_version}${image_suffix}${seed_suffix}"
        image_exists $image_tag_with_version
        # Get the exit code of the command
        exit_code=$?

        # if exit code is different than 0, therefore the last command failed, because the image is not found
        if [ $exit_code -ne 0 ]; then
            echo "image: ($image_tag_with_version) does not exists"
            docker tag $image_tag_with_commit_hash $image_tag_with_version
            docker push $image_tag_with_version
        fi
    done

    # Create and push the joint manifest
    create_and_push_manifest $IMAGE $CIRCLE_SHA1 $seed_suffix

    image_manifest_with_version="$IMAGE:${bin_crate_version}${seed_suffix}"
    image_exists $image_manifest_with_version
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        echo "manifest: ($image_manifest_with_version) does not exists"
        create_and_push_manifest $IMAGE $bin_crate_version $seed_suffix

        echo "Creating and pushing the joint manifest with the latest tag"
        create_and_push_manifest $IMAGE "latest" $seed_suffix
    else
        echo "manifest: ($image_manifest_with_version) exists"
    fi
done