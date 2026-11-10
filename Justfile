raspi := "borba@rasplab"
raspi_dir := "~/rasplab"

# lista os recipes disponíveis
default:
    @just --list

# compila o binário para aarch64 (bare metal)
build:
    cargo build --release

# compila a imagem Docker para arm64
docker-build:
    docker buildx build --platform linux/arm64 -t rasplab:latest --load .

# transfere a imagem para o Raspi e sobe o container
docker-deploy: docker-build
    docker save rasplab:latest | ssh {{raspi}} 'docker load'
    ssh {{raspi}} 'cd {{raspi_dir}} && docker compose up -d --force-recreate'

# copia o binário para o Raspi (deploy sem Docker)
deploy: build
    ssh {{raspi}} 'mkdir -p {{raspi_dir}}'
    scp target/aarch64-unknown-linux-musl/release/rasplab {{raspi}}:{{raspi_dir}}/rasplab

# segue os logs do container no Raspi
logs:
    ssh {{raspi}} 'docker compose -p {{raspi_dir}} logs -f'

# status do container no Raspi
status:
    ssh {{raspi}} 'docker compose -p {{raspi_dir}} ps'
