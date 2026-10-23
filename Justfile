set shell := ["bash", "-cu"]

name   := "rasplab"
target := "aarch64-unknown-linux-gnu"
pi     := "borba@192.168.1.15"
remote := "/home/borba/" + name
bin    := "target/" + target + "/release/" + name

# Mostra o menu de comandos disponíveis
default:
    @just --list --unsorted

# Instala o target aarch64 no rustup (rode uma vez)
setup:
    rustup target add {{target}}

# Compila para o Raspberry Pi 3
build:
    cargo build --release --target {{target}}

# Confere a arquitetura do Pi (deve mostrar aarch64)
check-pi:
    ssh {{pi}} uname -m

# Envia o binário para o Pi
deploy: build
    scp {{bin}} {{pi}}:{{remote}}

# Envia e executa no Pi (Ctrl+C para parar)
run: deploy
    ssh -t {{pi}} {{remote}}

# Remove artefatos de build
clean:
    cargo clean

# Menu interativo: escolha a ação pelo número
menu:
    #!/usr/bin/env bash
    set -euo pipefail
    options=(
      "setup    - instala o target aarch64"
      "build    - compila para o Pi 3"
      "check-pi - testa SSH e arquitetura do Pi"
      "deploy   - compila e envia o binário"
      "run      - envia e executa no Pi"
      "clean    - remove artefatos"
      "sair"
    )
    PS3="Escolha uma ação: "
    select opt in "${options[@]}"; do
      case "$REPLY" in
        1) just setup ;;
        2) just build ;;
        3) just check-pi ;;
        4) just deploy ;;
        5) just run ;;
        6) just clean ;;
        7) break ;;
        *) echo "Opção inválida" ;;
      esac
      break
    done
