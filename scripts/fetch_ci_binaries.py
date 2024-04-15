#!/usr/bin/env python

"""Fetch executable binary artifacts from the latest Circle CI workflow on main"""

import os
import sys
from typing import List, Optional, Tuple
import requests

HTTP_TIMEOUT = 5

DEVNET_CI_URL = (
    "https://circleci.com/api/v1.1/project/github/0xSpaceShard/starknet-devnet-rs"
)

BINARY_BUILD_JOB_PREFIX = "binary-build-"


def get_artifact_info(job: dict) -> Optional[Tuple[str, str]]:
    """
    Returns the artifact (URL, name) corresponding to job object.
    Returns None if job not a binary build.
    """
    if job["job_name"].startswith("binary-build-"):
        job_id = job["job_id"]
        job_name = job["job_name"]
        artifact_name = (
            job_name.replace(BINARY_BUILD_JOB_PREFIX, "starknet-devnet-") + ".tar.gz"
        )
        return (
            f"https://output.circle-artifacts.com/output/job/{job_id}/{artifact_name}",
            artifact_name,
        )

    raise ValueError(f"Invalid job: {job}")


def write_artifacts(workflow_id: str, artifact_infos: List[Tuple[str, str]]):
    """Write artifacts on disk"""
    os.mkdir(workflow_id)

    for artifact_url, artifact_name in artifact_infos:
        artifact_resp = requests.get(artifact_url, timeout=HTTP_TIMEOUT)
        target_path = os.path.join(workflow_id, artifact_name)
        with open(target_path, "wb") as f:
            f.write(artifact_resp.content)
            print(target_path)


def log_pipeline(workflow_id: str, pipeline: dict):
    """Log pipeline info"""
    print("ID:", workflow_id)
    print("Time finished:", pipeline["stop_time"])
    print("Status:", pipeline["status"])

    commit_details = pipeline["all_commit_details"]
    assert len(commit_details) == 1  # seems to always be one
    print("Commit SHA:", commit_details[0]["commit"])


def main():
    """Main functionality"""

    print("Fetching binaries from the latest main workflow")

    pipelines = requests.get(DEVNET_CI_URL, timeout=HTTP_TIMEOUT).json()
    for pipeline in pipelines:
        # break from the latest workflow on main; assuming descending order
        if pipeline["branch"] == "main":
            workflow_id = pipeline["workflows"]["workflow_id"]
            log_pipeline(workflow_id, pipeline)
            break
    else:
        sys.exit("Error: Could not locate workflow ID")

    jobs = [
        pipeline["workflows"]
        for pipeline in pipelines
        if pipeline["workflows"]["workflow_id"] == workflow_id
    ]

    artifact_infos = [
        get_artifact_info(job)
        for job in jobs
        if job["job_name"].startswith(BINARY_BUILD_JOB_PREFIX)
    ]

    if not artifact_infos:
        sys.exit("Error: No artifacts found")

    write_artifacts(workflow_id, artifact_infos)


if __name__ == "__main__":
    main()
