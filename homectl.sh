#!/bin/bash
# =============================================
# Script de Manutenção - Homelab Raspberry Pi 3
# =============================================

COMPOSE_FILE="docker-compose.yaml"
PROJECT_NAME="homelab"

cd "$(dirname "$0")" || exit

case "$1" in
    up|start)
        echo "🚀 Iniciando serviços Docker..."
        docker compose -f $COMPOSE_FILE up -d
        echo "✅ Serviços iniciados!"
        ;;

    down|stop)
        echo "⛔ Parando serviços Docker..."
        docker compose -f $COMPOSE_FILE down
        echo "✅ Serviços parados!"
        ;;

    restart)
        echo "🔄 Reiniciando serviços Docker..."
        docker compose -f $COMPOSE_FILE restart
        echo "✅ Serviços reiniciados!"
        ;;

    limpeza|clean|limpar)
        echo "🧹 Fazendo limpeza Docker..."
        docker compose -f $COMPOSE_FILE down
        docker system prune -af --volumes
        echo "✅ Limpeza completa!"
        ;;

    atualizacao|update|atualizar)
        echo "📦 Atualizando imagens Docker..."
        docker compose -f $COMPOSE_FILE pull
        docker compose -f $COMPOSE_FILE up -d
        docker image prune -f
        echo "✅ Atualização concluída!"
        ;;

    gpg-config|gpg)
        echo "🔑 Configurando GPG-Agent (cache de senha)..."
        cat > ~/.gnupg/gpg-agent.conf << EOF
pinentry-program /usr/bin/pinentry-tty
allow-loopback-pinentry
default-cache-ttl 86400
max-cache-ttl 86400
EOF
        chmod 700 ~/.gnupg
        chmod 600 ~/.gnupg/*
        gpgconf --kill gpg-agent
        echo 'export GPG_TTY=$(tty)' >> ~/.bashrc
        source ~/.bashrc
        echo "✅ GPG configurado! (senha cacheada por 24h)"
        ;;

    git-status)
        echo "📊 Status do Git:"
        git status
        ;;

    git-pull)
        echo "⬇️  Fazendo git pull..."
        git pull
        ;;

    git-push)
        echo "⬆️  Fazendo git push..."
        git push
        ;;

    git-log)
        echo "📜 Últimos commits:"
        git log --oneline -10
        ;;

    logs)
        echo "📜 Logs Docker (Ctrl+C para sair)..."
        docker compose -f $COMPOSE_FILE logs -f
        ;;

    status|ps)
        echo "📡 Status dos containers:"
        docker compose -f $COMPOSE_FILE ps
        ;;

    *)
        echo "══════════════════════════════════════════════"
        echo "   Script de Manutenção - Homelab"
        echo "══════════════════════════════════════════════"
        echo ""
        echo "Uso: ./manutencao.sh [comando]"
        echo ""
        echo "Docker:"
        echo "  up            → Inicia serviços"
        echo "  down          → Para serviços"
        echo "  restart       → Reinicia serviços"
        echo "  atualizacao   → Atualiza todas as imagens"
        echo "  limpeza       → Limpeza profunda"
        echo ""
        echo "GPG:"
        echo "  gpg-config    → Configura cache de senha PGP"
        echo ""
        echo "Git:"
        echo "  git-status    → Status do repositório"
        echo "  git-pull      → Pull do repositório"
        echo "  git-push      → Push do repositório"
        echo "  git-log       → Últimos 10 commits"
        echo ""
        echo "Outros:"
        echo "  logs          → Ver logs em tempo real"
        echo "  status        → Status dos containers"
        echo ""
        ;;
esac

