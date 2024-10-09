#!/bin/bash
# Run the test job
act --container-architecture linux/amd64 -j test -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest
