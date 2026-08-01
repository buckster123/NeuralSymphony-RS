#!/bin/bash
# Neural Symphony Studio - Quick Start Script
# Runs both backend and frontend

set -e

echo "🎵 Neural Symphony Studio - Starting..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Check Python
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}❌ Python3 not found${NC}"
    exit 1
fi

# Check Node
if ! command -v node &> /dev/null; then
    echo -e "${RED}❌ Node.js not found${NC}"
    exit 1
fi

# Setup backend
echo -e "${CYAN}📦 Setting up backend...${NC}"
cd backend
if [ ! -d "venv" ]; then
    echo "   Creating virtual environment..."
    python3 -m venv venv
fi

source venv/bin/activate
echo "   Installing dependencies..."
pip install -q -r requirements.txt

# Check Cerebro
if [ ! -f "$HOME/.local/bin/cerebro-mcp" ]; then
    echo -e "${YELLOW}⚠️  Cerebro MCP not found at ~/.local/bin/cerebro-mcp${NC}"
    echo "   Make sure CerebroCortex-RS is installed"
else
    echo -e "${GREEN}   ✅ Cerebro MCP found${NC}"
fi

# Setup frontend
echo ""
echo -e "${CYAN}📦 Setting up frontend...${NC}"
cd ../frontend
if [ ! -d "node_modules" ]; then
    echo "   Installing npm packages..."
    npm install --silent
fi

# Start services
echo ""
echo -e "${GREEN}🚀 Starting services...${NC}"
echo ""

# Terminal 1: Backend
echo -e "${BLUE}┌──────────────────────────────────────────────┐${NC}"
echo -e "${BLUE}│  Backend: http://localhost:8765              │${NC}"
echo -e "${BLUE}└──────────────────────────────────────────────┘${NC}"
cd ../backend
source venv/bin/activate
uvicorn main:app --reload --host 0.0.0.0 --port 8765 &
BACKEND_PID=$!

# Wait for backend
sleep 2

# Terminal 2: Frontend
echo -e "${BLUE}┌──────────────────────────────────────────────┐${NC}"
echo -e "${BLUE}│  Frontend: http://localhost:5173             │${NC}"
echo -e "${BLUE}└──────────────────────────────────────────────┘${NC}"
cd ../frontend
npm run dev &
FRONTEND_PID=$!

echo ""
echo -e "${GREEN}✅ Neural Symphony Studio is running!${NC}"
echo ""
echo -e "Open your browser: ${CYAN}http://localhost:5173${NC}"
echo ""
echo -e "${YELLOW}To stop:${NC} Press Ctrl+C twice (kills both processes)"
echo ""

# Wait for interrupt
trap "kill $BACKEND_PID $FRONTEND_PID 2>/dev/null; exit" INT TERM

wait