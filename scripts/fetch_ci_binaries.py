#!/usr/bin/env python

"""Fetch executable binary artifacts from a Circle CI workflow"""

import argparse
import os
import sys
from typing import List, Optional, Tuple
import requests

HTTP_TIMEOUT = 5

DEVNET_CI_URL = (
    "https://circleci.com/api/v1.1/project/github/0xSpaceShard/starknet-devnet-rs"
)

ARTIFACT_URL_TEMPLATE = (
    "https://output.circle-artifacts.com/output/job/{}/artifacts/0/{}"
)

BINARY_BUILD_JOB_PREFIX = "binary-build-"

WARNING_COLOR = "\033[93m"
END_COLOR = "\033[0m"


def get_artifact_info(job: dict) -> Optional[Tuple[str, str]]:
    """
    Returns the artifact (URL, name) corresponding to job object.
    Returns None if job is not a binary build.
    """
    if job["job_name"].startswith("binary-build-"):
        job_id = job["job_id"]
        job_name = job["job_name"]
        artifact_name = (
            job_name.replace(BINARY_BUILD_JOB_PREFIX, "starknet-devnet-") + ".tar.gz"
        )
        return ARTIFACT_URL_TEMPLATE.format(job_id, artifact_name), artifact_name

    raise ValueError(f"Invalid job: {job}")


def write_artifacts(workflow_id: str, artifact_infos: List[Tuple[str, str]]):
    """Write artifacts on disk"""
    if os.path.exists(workflow_id):
        print(
            WARNING_COLOR
            + f"Warning: Directory {workflow_id} already exists, overwriting its content"
            + END_COLOR,
            file=sys.stderr,
        )
    else:
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
    print("Workflow finished at:", pipeline["stop_time"])
    print("Workflow status:", pipeline["status"])

    commit_details = pipeline["all_commit_details"]
    assert len(commit_details) == 1  # seems to always be one
    print("Commit SHA:", commit_details[0]["commit"])


PARSER = argparse.ArgumentParser(prog=__file__, description=__doc__)
PARSER.add_argument(
    "-w",
    "--workflow",
    metavar="ID",
    help="""Specify the ID of the workflow whose artifacts you want to fetch. \
The ID can be found in the CircleCI URL: \
https://app.circleci.com/pipelines/github/0xSpaceShard/starknet-devnet-rs/1742/workflows/2c521658-35a7-4f15-a760-af5042491d35. \
If omitted, defaults to the latest successful workflow on main""",
)


def get_workflow_predicate(workflow_arg: Optional[str]):
    """
    Get predicate for identifying the desired workflow.
    If no user arg, defaults to the latest successful main workflow.
    """
    if workflow_arg:
        print("Fetching binaries from workflow with ID:", workflow_arg)
        return lambda pipeline: pipeline["workflows"]["workflow_id"] == workflow_arg

    print("Fetching binaries from the latest successful workflow on main")
    return (
        lambda pipeline: pipeline["branch"] == "main"
        and pipeline["status"] == "success"
    )


def main():
    """Main functionality"""

    args = PARSER.parse_args()
    workflow_predicate = get_workflow_predicate(args.workflow)

    pipelines = requests.get(DEVNET_CI_URL, timeout=HTTP_TIMEOUT).json()
    for pipeline in pipelines:
        # assuming a chronologically descending order
        if workflow_predicate(pipeline):
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
