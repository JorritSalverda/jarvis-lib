#!/bin/bash
set -e

DOCKER_BUILDKIT=1 docker build \
	--target planner \
	--tag jsalverda/jarvis-lib-builder:dlc-main-planner \
	--cache-from jsalverda/jarvis-lib-builder:dlc-main-planner \
	--build-arg BUILDKIT_INLINE_CACHE=1 .
DOCKER_BUILDKIT=1 docker push jsalverda/jarvis-lib-builder:dlc-main-planner

DOCKER_BUILDKIT=1 docker build \
	--target cacher \
	--tag jsalverda/jarvis-lib-builder:dlc-main-cacher \
	--cache-from jsalverda/jarvis-lib-builder:dlc-main-planner \
	--cache-from jsalverda/jarvis-lib-builder:dlc-main-cacher \
	--build-arg BUILDKIT_INLINE_CACHE=1 .
DOCKER_BUILDKIT=1 docker push jsalverda/jarvis-lib-builder:dlc-main-cacher

DOCKER_BUILDKIT=1 docker build \
	--target builder \
	--tag jsalverda/jarvis-lib-builder:dlc-main-builder \
	--cache-from jsalverda/jarvis-lib-builder:dlc-main-planner \
	--cache-from jsalverda/jarvis-lib-builder:dlc-main-cacher \
	--cache-from jsalverda/jarvis-lib-builder:dlc-main-builder \
	--build-arg BUILDKIT_INLINE_CACHE=1 .
DOCKER_BUILDKIT=1 docker push jsalverda/jarvis-lib-builder:dlc-main-builder