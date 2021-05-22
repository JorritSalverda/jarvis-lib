#!/bin/bash
set -e

DOCKER_BUILDKIT=1 docker build \
	--tag jsalverda/jarvis-lib:dlc \
	--cache-from jsalverda/jarvis-lib:dlc \
	--build-arg BUILDKIT_INLINE_CACHE=1 .
DOCKER_BUILDKIT=1 docker push jsalverda/jarvis-lib:dlc