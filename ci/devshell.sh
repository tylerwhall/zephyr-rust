#!/bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
. "${DIR}/env.sh"

exec docker run \
    -it --rm \
    -v /etc/passwd:/etc/passwd:ro \
    -v /etc/group:/etc/group:ro \
    -v /etc/shadow:/etc/shadow:ro \
    -v ${DIR}/..:/zephyr-rust \
    -w /zephyr-rust \
    ${CONTAINER_REGISTRY}${ZEPHYR_VERSION}-${RUST_VERSION} \
    sh -c "chown -R $USER:$GROUP /zephyrproject && \
           chown -R $USER:$GROUP \$CARGO_HOME && \
           mkdir -p $HOME && \
           chown $USER:$GROUP $HOME && \
           su -s /bin/bash $USER"
