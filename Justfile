raspi := "borba@rasplab"
raspi_dir := "~/rasplab"

# list available recipes
default:
    @just --list

# build the binary for aarch64 (bare metal)
build:
    cargo build --release

# format code with rustfmt
fmt:
    cargo fmt --all

# lint with clippy
clippy:
    cargo clippy --all-targets

# apply automatic clippy/compiler fixes then format
fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets
    cargo fmt --all

# remove build artifacts
clean:
    cargo clean

# build the Docker image for arm64
docker-build:
    docker buildx build --platform linux/arm64 -t rasplab:latest --load .

# build, transfer image to Raspi and start the container
docker-deploy: docker-build
    docker save rasplab:latest | ssh {{raspi}} 'docker load'
    ssh {{raspi}} 'cd {{raspi_dir}} && docker compose up -d --force-recreate'

# copy the binary to the Raspi (Docker-free deploy)
deploy: build
    ssh {{raspi}} 'mkdir -p {{raspi_dir}}'
    scp target/aarch64-unknown-linux-musl/release/rasplab {{raspi}}:{{raspi_dir}}/rasplab

# follow container logs on the Raspi
logs:
    ssh {{raspi}} 'cd {{raspi_dir}} && docker compose logs -f'

# show container status on the Raspi
status:
    ssh {{raspi}} 'cd {{raspi_dir}} && docker compose ps'
