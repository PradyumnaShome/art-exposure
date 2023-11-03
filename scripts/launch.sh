#!/bin/bash
PROJECT_DIR="$(cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )/.."

${PROJECT_DIR}/target/release/art-exposure "Henri Matisse" "${PROJECT_DIR}/fonts/PlayfairDisplay-Regular.ttf"
