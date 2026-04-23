#!/bin/bash
# MC-Minder AI Chat Diagnostic Script
# AI 聊天功能诊断脚本
# Usage: bash scripts/diagnose_ai.sh [config_path]

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

CONFIG_FILE="${1:-config.toml}"
LOG_DIR="logs"
LOG_FILE="$LOG_DIR/mc-minder.log"
SERVER_LOG="$LOG_DIR/latest.log"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  MC-Minder AI Chat Diagnostic Tool${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Track issues
ISSUES=0
WARNINGS=0

check_pass() {
    echo -e "  ${GREEN}[PASS]${NC} $1"
}

check_fail() {
    echo -e "  ${RED}[FAIL]${NC} $1"
    ((ISSUES++))
}

check_warn() {
    echo -e "  ${YELLOW}[WARN]${NC} $1"
    ((WARNINGS++))
}

# ============================================
# 1. Check config file exists
# ============================================
echo -e "${BLUE}[1/8] Checking config file...${NC}"
if [ -f "$CONFIG_FILE" ]; then
    check_pass "Config file found: $CONFIG_FILE"
else
    check_fail "Config file not found: $CONFIG_FILE"
    echo ""
    echo -e "${RED}Cannot continue without config file. Run: mc-minder init${NC}"
    exit 1
fi

# ============================================
# 2. Check AI configuration
# ============================================
echo ""
echo -e "${BLUE}[2/8] Checking AI configuration...${NC}"

# Check if [ai] section exists
if grep -q "^\[ai\]" "$CONFIG_FILE"; then
    check_pass "AI section found in config"
    
    # Check api_url
    API_URL=$(grep "^api_url" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?/\1/' | tr -d ' ')
    if [ -n "$API_URL" ] && [ "$API_URL" != '""' ]; then
        check_pass "api_url is set: $API_URL"
    else
        check_warn "api_url is empty - AI will use Ollama or be disabled"
    fi
    
    # Check api_key
    API_KEY=$(grep "^api_key" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?/\1/' | tr -d ' ')
    if [ -n "$API_KEY" ] && [ "$API_KEY" != '""' ]; then
        check_pass "api_key is set (hidden for security)"
    else
        if [ -n "$API_URL" ] && [ "$API_URL" != '""' ]; then
            check_fail "api_key is empty but api_url is set - API calls will fail"
        else
            check_warn "api_key is empty (OK if using Ollama)"
        fi
    fi
    
    # Check trigger
    TRIGGER=$(grep "^trigger" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?/\1/' | tr -d ' ')
    if [ -n "$TRIGGER" ]; then
        check_pass "Trigger is set: '$TRIGGER'"
    else
        check_warn "Trigger is empty - defaulting to '!'"
    fi
    
    # Check model
    MODEL=$(grep "^model" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?/\1/' | tr -d ' ')
    if [ -n "$MODEL" ]; then
        check_pass "Model is set: $MODEL"
    else
        check_warn "Model is empty - using default"
    fi
    
else
    check_warn "No [ai] section in config - AI features disabled"
fi

# Check Ollama config
echo ""
echo -e "${BLUE}[3/8] Checking Ollama configuration...${NC}"

if grep -q "^\[ollama\]" "$CONFIG_FILE"; then
    check_pass "Ollama section found in config"
    
    OLLAMA_ENABLED=$(grep "^enabled" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*\(true\|false\)/\1/' | tr -d ' ')
    if [ "$OLLAMA_ENABLED" = "true" ]; then
        check_pass "Ollama is enabled"
        
        OLLAMA_URL=$(grep "^url" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?/\1/' | tr -d ' ')
        if [ -n "$OLLAMA_URL" ]; then
            check_pass "Ollama URL: $OLLAMA_URL"
            
            # Test Ollama connection
            echo -e "  Testing Ollama connection..."
            OLLAMA_HEALTH=$(echo "$OLLAMA_URL" | sed 's|/api/generate|/api/tags|')
            if curl -s --connect-timeout 3 "$OLLAMA_HEALTH" > /dev/null 2>&1; then
                check_pass "Ollama is reachable"
            else
                check_fail "Cannot connect to Ollama at $OLLAMA_URL"
            fi
        else
            check_fail "Ollama URL is empty"
        fi
    else
        check_pass "Ollama is disabled (using OpenAI-compatible API)"
    fi
else
    check_pass "No Ollama section (using OpenAI-compatible API)"
fi

# ============================================
# 3. Check RCON configuration
# ============================================
echo ""
echo -e "${BLUE}[4/8] Checking RCON configuration...${NC}"

if grep -q "^\[rcon\]" "$CONFIG_FILE"; then
    check_pass "RCON section found in config"
    
    RCON_HOST=$(grep "^host" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?/\1/' | tr -d ' ')
    RCON_PORT=$(grep "^port" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*\([0-9]*\)/\1/' | tr -d ' ')
    RCON_PASS=$(grep "^password" "$CONFIG_FILE" | head -1 | sed 's/.*=\s*"\?\([^"]*\)"\?/\1/' | tr -d ' ')
    
    check_pass "RCON Host: ${RCON_HOST:-127.0.0.1}"
    check_pass "RCON Port: ${RCON_PORT:-25575}"
    
    if [ -n "$RCON_PASS" ] && [ "$RCON_PASS" != '""' ]; then
        check_pass "RCON password is set"
    else
        check_fail "RCON password is empty - AI responses cannot be sent to players"
    fi
else
    check_fail "No [rcon] section in config"
fi

# ============================================
# 4. Check log directory and files
# ============================================
echo ""
echo -e "${BLUE}[5/8] Checking log files...${NC}"

if [ -d "$LOG_DIR" ]; then
    check_pass "Log directory exists: $LOG_DIR"
else
    check_warn "Log directory not found: $LOG_DIR (will be created on start)"
fi

if [ -f "$LOG_FILE" ]; then
    LOG_SIZE=$(stat -f%z "$LOG_FILE" 2>/dev/null || stat -c%s "$LOG_FILE" 2>/dev/null || echo "unknown")
    check_pass "MC-Minder log exists: $LOG_FILE ($LOG_SIZE bytes)"
else
    check_warn "MC-Minder log not found: $LOG_FILE"
fi

if [ -f "$SERVER_LOG" ]; then
    SERVER_SIZE=$(stat -f%z "$SERVER_LOG" 2>/dev/null || stat -c%s "$SERVER_LOG" 2>/dev/null || echo "unknown")
    check_pass "Server log exists: $SERVER_LOG ($SERVER_SIZE bytes)"
else
    check_warn "Server log not found: $SERVER_LOG (start Minecraft server first)"
fi

# ============================================
# 5. Check for AI-related log entries
# ============================================
echo ""
echo -e "${BLUE}[6/8] Analyzing AI log entries...${NC}"

if [ -f "$LOG_FILE" ]; then
    AI_LOG_COUNT=$(grep -c "\[AI\]" "$LOG_FILE" 2>/dev/null || echo "0")
    if [ "$AI_LOG_COUNT" -gt 0 ]; then
        check_pass "Found $AI_LOG_COUNT AI-related log entries"
        
        # Check for errors
        AI_ERRORS=$(grep -c "\[AI\].*error\|error.*\[AI\]" "$LOG_FILE" 2>/dev/null || echo "0")
        if [ "$AI_ERRORS" -gt 0 ]; then
            check_fail "Found $AI_ERRORS AI error entries:"
            grep -i "\[AI\].*error\|error.*\[AI\]" "$LOG_FILE" | tail -3 | while read line; do
                echo -e "    ${RED}▸${NC} $line"
            done
        else
            check_pass "No AI errors found in log"
        fi
        
        # Check for successful responses
        AI_SUCCESS=$(grep -c "\[AI\] Received response\|\[AI\] Successfully sent" "$LOG_FILE" 2>/dev/null || echo "0")
        if [ "$AI_SUCCESS" -gt 0 ]; then
            check_pass "Found $AI_SUCCESS successful AI interactions"
        else
            check_warn "No successful AI interactions logged"
        fi
        
        # Check for trigger detection
        TRIGGER_HITS=$(grep -c "\[AI\] Trigger detected\|\[AI\] Checking trigger" "$LOG_FILE" 2>/dev/null || echo "0")
        if [ "$TRIGGER_HITS" -gt 0 ]; then
            check_pass "Trigger detection working ($TRIGGER_HITS hits)"
        else
            check_warn "No trigger detection logged - players may not be using AI"
        fi
    else
        check_warn "No AI log entries found - AI may not be configured or used"
    fi
    
    # Check RCON logs
    RCON_LOG=$(grep -c "RCON" "$LOG_FILE" 2>/dev/null || echo "0")
    if [ "$RCON_LOG" -gt 0 ]; then
        RCON_ERRORS=$(grep -c "RCON.*error\|error.*RCON\|RCON.*fail" "$LOG_FILE" 2>/dev/null || echo "0")
        if [ "$RCON_ERRORS" -gt 0 ]; then
            check_fail "Found $RCON_ERRORS RCON errors:"
            grep -i "RCON.*error\|error.*RCON\|RCON.*fail" "$LOG_FILE" | tail -3 | while read line; do
                echo -e "    ${RED}▸${NC} $line"
            done
        else
            check_pass "No RCON errors in log"
        fi
    fi
else
    check_warn "Cannot analyze logs - file not found"
fi

# ============================================
# 6. Check server log for chat messages
# ============================================
echo ""
echo -e "${BLUE}[7/8] Checking server chat messages...${NC}"

if [ -f "$SERVER_LOG" ]; then
    # Check for chat messages with trigger
    TRIGGER_CHAR="${TRIGGER:-!}"
    CHAT_COUNT=$(grep -c "<.*> $TRIGGER_CHAR" "$SERVER_LOG" 2>/dev/null || echo "0")
    if [ "$CHAT_COUNT" -gt 0 ]; then
        check_pass "Found $CHAT_COUNT chat messages with trigger '$TRIGGER_CHAR'"
        echo -e "  ${BLUE}Recent trigger messages:${NC}"
        grep "<.*> $TRIGGER_CHAR" "$SERVER_LOG" | tail -3 | while read line; do
            echo -e "    ▸ $line"
        done
    else
        check_warn "No chat messages with trigger '$TRIGGER_CHAR' found"
    fi
else
    check_warn "Cannot check server log - file not found"
fi

# ============================================
# 7. Check verbose mode
# ============================================
echo ""
echo -e "${BLUE}[8/8] Checking verbose mode...${NC}"

if [ "$1" = "-v" ] || [ "$1" = "--verbose" ]; then
    check_pass "Verbose mode enabled"
else
    check_warn "Run with -v for verbose debug output"
fi

# ============================================
# Summary
# ============================================
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Diagnostic Summary${NC}"
echo -e "${BLUE}========================================${NC}"

if [ "$ISSUES" -eq 0 ] && [ "$WARNINGS" -eq 0 ]; then
    echo -e "  ${GREEN}All checks passed!${NC}"
    echo -e "  AI chat should be working correctly."
elif [ "$ISSUES" -eq 0 ]; then
    echo -e "  ${YELLOW}Found $WARNINGS warning(s)${NC}"
    echo -e "  AI chat may work but review warnings above."
else
    echo -e "  ${RED}Found $ISSUES issue(s) and $WARNINGS warning(s)${NC}"
    echo -e "  Please fix the issues above before using AI chat."
fi

echo ""
echo -e "${BLUE}Troubleshooting Tips:${NC}"
echo -e "  1. Enable verbose logging: ./mc-minder -v"
echo -e "  2. Watch logs in real-time: tail -f $LOG_FILE"
echo -e "  3. Check trigger config: grep trigger $CONFIG_FILE"
echo -e "  4. Test RCON: rcon -H ${RCON_HOST:-127.0.0.1} -p ${RCON_PORT:-25575} -P <password> list"
echo ""

exit $ISSUES
