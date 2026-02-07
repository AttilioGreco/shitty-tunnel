#!/usr/bin/env bash
# Script di validazione configurazione Docker

set -e

echo "🔍 Validazione configurazione Docker..."
echo ""

# Colori
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check files exist
echo "1️⃣  Verifica file esistenti..."
for file in server.toml client.toml traefik.yml traefik-dynamic.yml; do
    if [ -f "docker/$file" ]; then
        echo -e "${GREEN}✓${NC} $file"
    else
        echo -e "${RED}✗${NC} $file - MANCANTE!"
        exit 1
    fi
done
echo ""

# Extract keys
echo "2️⃣  Estrazione chiavi..."
SERVER_PRIV=$(grep "^private_key" docker/server.toml | cut -d'"' -f2)
SERVER_PUB_IN_CLIENT=$(grep "^server_public_key" docker/client.toml | cut -d'"' -f2)
CLIENT_PRIV=$(grep "^private_key" docker/client.toml | cut -d'"' -f2)
CLIENT_PUB_IN_SERVER=$(grep "^public_key" docker/server.toml | cut -d'"' -f2)

echo "Server private: ${SERVER_PRIV:0:20}..."
echo "Client private: ${CLIENT_PRIV:0:20}..."
echo ""

# Check key lengths (base64 Ed25519 = 44 chars)
echo "3️⃣  Verifica lunghezza chiavi (devono essere 44 caratteri base64)..."
if [ ${#SERVER_PRIV} -eq 44 ]; then
    echo -e "${GREEN}✓${NC} Server private key: 44 caratteri"
else
    echo -e "${RED}✗${NC} Server private key: ${#SERVER_PRIV} caratteri (dovrebbe essere 44)"
    exit 1
fi

if [ ${#CLIENT_PRIV} -eq 44 ]; then
    echo -e "${GREEN}✓${NC} Client private key: 44 caratteri"
else
    echo -e "${RED}✗${NC} Client private key: ${#CLIENT_PRIV} caratteri (dovrebbe essere 44)"
    exit 1
fi
echo ""

# Check domain
echo "4️⃣  Verifica domain..."
DOMAIN=$(grep "^domain" docker/server.toml | cut -d'"' -f2)
if [[ "$DOMAIN" == *":"* ]]; then
    echo -e "${YELLOW}⚠${NC}  Domain contiene porta: $DOMAIN"
    echo "   Raccomandato: rimuovere la porta (es: test.localhost)"
else
    echo -e "${GREEN}✓${NC} Domain: $DOMAIN"
fi
echo ""

# Check server_host
echo "5️⃣  Verifica server_host..."
SERVER_HOST=$(grep "^server_host" docker/client.toml | cut -d'"' -f2)
if [[ "$SERVER_HOST" == *"server:"* ]]; then
    echo -e "${GREEN}✓${NC} Server host: $SERVER_HOST (usa nome container Docker)"
else
    echo -e "${YELLOW}⚠${NC}  Server host: $SERVER_HOST"
    echo "   Raccomandato: http://server:8443"
fi
echo ""

# Check Traefik backend
echo "6️⃣  Verifica backend Traefik..."
TRAEFIK_BACKEND=$(grep "url:" docker/traefik-dynamic.yml | awk '{print $3}' | tr -d '"')
if [[ "$TRAEFIK_BACKEND" == *"server:"* ]]; then
    echo -e "${GREEN}✓${NC} Traefik backend: $TRAEFIK_BACKEND"
else
    echo -e "${RED}✗${NC} Traefik backend: $TRAEFIK_BACKEND"
    echo "   Dovrebbe essere: http://server:8080"
    exit 1
fi
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Configurazione valida!${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Puoi avviare con: just up"
echo ""
