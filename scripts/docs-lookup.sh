#!/usr/bin/env bash

# Symbion Documentation Lookup Script
# Quick access to technical documentation

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)/docs"

# Helper functions
print_header() {
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1" >&2
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Main menu
show_menu() {
    print_header "📚 Symbion Documentation"
    echo ""
    echo "Quick access commands:"
    echo ""
    echo -e "  ${GREEN}quick${NC}            Quick reference (cheat sheet)"
    echo -e "  ${GREEN}endpoints${NC}        List all HTTP API endpoints"
    echo -e "  ${GREEN}mqtt${NC}             List all MQTT topics"
    echo -e "  ${GREEN}security${NC}         Security mechanisms summary"
    echo -e "  ${GREEN}auth${NC}             Authentication guide"
    echo -e "  ${GREEN}webauthn${NC}         WebAuthn/Passkeys guide (biometric)"
    echo -e "  ${GREEN}contracts${NC}        MQTT contracts (schemas)"
    echo -e "  ${GREEN}flows${NC}            MQTT message flows"
    echo -e "  ${GREEN}agents${NC}           Agent management endpoints"
    echo -e "  ${GREEN}context${NC}          Context engine endpoints"
    echo -e "  ${GREEN}notes${NC}            Notes/Memo endpoints"
    echo -e "  ${GREEN}decision${NC}         Decision Engine endpoints"
    echo -e "  ${GREEN}mfa${NC}              MFA/TOTP endpoints"
    echo ""
    echo -e "  ${BLUE}view <file>${NC}      Open specific doc file"
    echo -e "  ${BLUE}search <term>${NC}    Search in documentation"
    echo -e "  ${BLUE}tree${NC}             Show documentation tree"
    echo ""
    echo "Examples:"
    echo -e "  ${YELLOW}./docs-lookup.sh quick${NC}"
    echo -e "  ${YELLOW}./docs-lookup.sh endpoints${NC}"
    echo -e "  ${YELLOW}./docs-lookup.sh webauthn${NC}"
    echo -e "  ${YELLOW}./docs-lookup.sh search \"JWT\"${NC}"
    echo -e "  ${YELLOW}./docs-lookup.sh view api/authentication.md${NC}"
}

# Extract section from markdown file
extract_section() {
    local file="$1"
    local section="$2"

    if [[ ! -f "$file" ]]; then
        print_error "File not found: $file"
        return 1
    fi

    # Extract section between headers
    awk -v section="$section" '
        BEGIN { found=0 }
        $0 ~ "^#{1,3} .*" section {
            found=1
            print
            next
        }
        found && $0 ~ /^#{1,3} / { exit }
        found { print }
    ' "$file"
}

# List all HTTP endpoints
list_endpoints() {
    print_header "🌐 HTTP API Endpoints"

    local endpoints_file="$DOCS_DIR/api/endpoints.md"

    if [[ ! -f "$endpoints_file" ]]; then
        print_error "Endpoints documentation not found"
        return 1
    fi

    # Extract table of contents
    grep -E "^### \`(GET|POST|PUT|DELETE)" "$endpoints_file" | sed 's/### //' | while read -r line; do
        if [[ $line =~ GET ]]; then
            echo -e "  ${GREEN}$line${NC}"
        elif [[ $line =~ POST ]]; then
            echo -e "  ${BLUE}$line${NC}"
        elif [[ $line =~ PUT ]]; then
            echo -e "  ${YELLOW}$line${NC}"
        elif [[ $line =~ DELETE ]]; then
            echo -e "  ${RED}$line${NC}"
        fi
    done

    echo ""
    print_info "Total endpoints: $(grep -c "^### \`" "$endpoints_file")"
    echo ""
    print_info "View full documentation: cat $endpoints_file | less"
}

# List all MQTT topics
list_mqtt_topics() {
    print_header "🔌 MQTT Topics"

    local topics_file="$DOCS_DIR/mqtt/topics.md"

    if [[ ! -f "$topics_file" ]]; then
        print_error "MQTT topics documentation not found"
        return 1
    fi

    # Extract topics
    grep -E "^### \`symbion/" "$topics_file" | sed 's/### //' | while read -r line; do
        if [[ $line =~ agents ]]; then
            echo -e "  ${GREEN}$line${NC}"
        elif [[ $line =~ notes ]]; then
            echo -e "  ${BLUE}$line${NC}"
        elif [[ $line =~ dashboard ]]; then
            echo -e "  ${YELLOW}$line${NC}"
        elif [[ $line =~ system ]]; then
            echo -e "  ${CYAN}$line${NC}"
        else
            echo -e "  $line"
        fi
    done

    echo ""
    print_info "Total topics: $(grep -c "^### \`symbion/" "$topics_file")"
    echo ""
    print_info "View full documentation: cat $topics_file | less"
}

# Show security summary
show_security() {
    print_header "🛡️ Security Mechanisms"

    local security_file="$DOCS_DIR/api/security.md"

    if [[ ! -f "$security_file" ]]; then
        print_error "Security documentation not found"
        return 1
    fi

    echo ""
    echo -e "${GREEN}1. TLS/HTTPS${NC}"
    echo "   - Port: 8443"
    echo "   - Certificats: /etc/mosquitto/certs/"
    echo "   - Endpoint CA: GET /ca-certificate"
    echo ""

    echo -e "${GREEN}2. CSRF Protection${NC}"
    echo "   - Nonces one-time (5 min TTL)"
    echo "   - Endpoint: GET /csrf-token"
    echo "   - Header requis: X-CSRF-Token"
    echo ""

    echo -e "${GREEN}3. Rate Limiting${NC}"
    echo "   - Login: 5 req/s (burst 10)"
    echo "   - API générale: 50 req/s (burst 100)"
    echo "   - Middleware: tower_governor"
    echo ""

    echo -e "${GREEN}4. CORS${NC}"
    echo "   - Origins whitelistées uniquement"
    echo "   - Credentials: true"
    echo "   - Max age: 3600s"
    echo ""

    echo -e "${GREEN}5. Input Validation${NC}"
    echo "   - Command whitelist (agents)"
    echo "   - ANSI escape sanitization"
    echo "   - JSON schema validation"
    echo ""

    print_info "View full security doc: cat $security_file | less"
}

# Show authentication guide
show_auth_guide() {
    print_header "🔐 Authentication Quick Guide"

    echo ""
    echo -e "${GREEN}1. JWT Authentication${NC}"
    echo "   POST /login → Receive token (24h)"
    echo "   Header: Authorization: Bearer <token>"
    echo ""

    echo -e "${GREEN}2. MFA/TOTP${NC}"
    echo "   POST /mfa/setup → QR code + backup codes"
    echo "   POST /mfa/verify-setup → Activate MFA"
    echo "   POST /login/mfa → Complete login with TOTP code"
    echo ""

    echo -e "${GREEN}3. WebAuthn Passkeys${NC}"
    echo "   POST /webauthn/register/start → Challenge"
    echo "   POST /webauthn/register/finish → Store passkey"
    echo "   POST /webauthn/auth/start → Login challenge"
    echo "   POST /webauthn/auth/finish → Complete login"
    echo ""

    echo -e "${GREEN}4. API Key (Fallback)${NC}"
    echo "   Header: X-Api-Key: <key>"
    echo "   Usage: Inter-service communication"
    echo ""

    local auth_file="$DOCS_DIR/api/authentication.md"
    print_info "View full auth doc: cat $auth_file | less"
}

# Show contracts summary
show_contracts() {
    print_header "📋 MQTT Contracts"

    local contracts_file="$DOCS_DIR/mqtt/contracts.md"

    if [[ ! -f "$contracts_file" ]]; then
        print_error "Contracts documentation not found"
        return 1
    fi

    echo ""
    echo "JSON Schema validation for MQTT messages:"
    echo ""

    grep -E "^### \`symbion/" "$contracts_file" | sed 's/### //' | while read -r topic; do
        echo -e "  ${CYAN}$topic${NC}"
    done

    echo ""
    print_info "View full contracts doc: cat $contracts_file | less"
}

# Show message flows
show_flows() {
    print_header "🔄 MQTT Message Flows"

    local flows_file="$DOCS_DIR/mqtt/flows.md"

    if [[ ! -f "$flows_file" ]]; then
        print_error "Flows documentation not found"
        return 1
    fi

    echo ""
    echo "Main communication patterns:"
    echo ""
    echo -e "  ${GREEN}Flow 1:${NC} Agent Lifecycle (Registration + Heartbeats)"
    echo -e "  ${GREEN}Flow 2:${NC} Agent Command Execution (Request-Response)"
    echo -e "  ${GREEN}Flow 3:${NC} Plugin RPC (Notes CRUD)"
    echo -e "  ${GREEN}Flow 4:${NC} Dashboard Real-Time Updates"
    echo ""

    print_info "View full flows doc: cat $flows_file | less"
}

# Show quick reference
show_quick_reference() {
    local quick_file="$DOCS_DIR/QUICK_REFERENCE.md"

    if [[ ! -f "$quick_file" ]]; then
        print_error "Quick reference not found"
        return 1
    fi

    print_header "🚀 Quick Reference"
    echo ""

    if command -v bat &> /dev/null; then
        bat --style=plain --paging=always "$quick_file"
    elif command -v less &> /dev/null; then
        less "$quick_file"
    else
        cat "$quick_file"
    fi
}

# Show WebAuthn guide
show_webauthn_guide() {
    local webauthn_file="$DOCS_DIR/api/webauthn.md"

    if [[ ! -f "$webauthn_file" ]]; then
        print_error "WebAuthn documentation not found"
        return 1
    fi

    print_header "🔐 WebAuthn / Passkeys Biométriques"
    echo ""

    if command -v bat &> /dev/null; then
        bat --style=plain --paging=always "$webauthn_file"
    elif command -v less &> /dev/null; then
        less "$webauthn_file"
    else
        cat "$webauthn_file"
    fi
}

# Show specific category endpoints
show_category_endpoints() {
    local category="$1"
    local endpoints_file="$DOCS_DIR/api/endpoints.md"

    if [[ ! -f "$endpoints_file" ]]; then
        print_error "Endpoints documentation not found"
        return 1
    fi

    case "$category" in
        agents)
            print_header "🤖 Agent Management Endpoints"
            extract_section "$endpoints_file" "Gestion Agents"
            ;;
        context)
            print_header "🎭 Context Engine Endpoints"
            extract_section "$endpoints_file" "Context Engine"
            ;;
        notes)
            print_header "📝 Notes/Memo Endpoints"
            extract_section "$endpoints_file" "Notes/Memo"
            ;;
        decision)
            print_header "🧠 Decision Engine Endpoints"
            extract_section "$endpoints_file" "Decision Engine"
            ;;
        mfa)
            print_header "🔑 MFA Endpoints"
            extract_section "$endpoints_file" "Multi-Factor Authentication"
            ;;
        *)
            print_error "Unknown category: $category"
            return 1
            ;;
    esac
}

# View specific documentation file
view_doc() {
    local file="$1"
    local full_path="$DOCS_DIR/$file"

    if [[ ! -f "$full_path" ]]; then
        print_error "Documentation file not found: $file"
        echo ""
        print_info "Available files:"
        find "$DOCS_DIR" -name "*.md" | sed "s|$DOCS_DIR/||" | while read -r f; do
            echo "  - $f"
        done
        return 1
    fi

    if command -v bat &> /dev/null; then
        bat --style=plain --paging=always "$full_path"
    elif command -v less &> /dev/null; then
        less "$full_path"
    else
        cat "$full_path"
    fi
}

# Search in documentation
search_docs() {
    local term="$1"

    print_header "🔍 Search Results for: $term"
    echo ""

    if ! command -v rg &> /dev/null && ! command -v grep &> /dev/null; then
        print_error "Neither ripgrep nor grep found"
        return 1
    fi

    if command -v rg &> /dev/null; then
        rg --color=always --heading --line-number --context 2 "$term" "$DOCS_DIR"
    else
        grep -r --color=always -n -C 2 "$term" "$DOCS_DIR"
    fi
}

# Show documentation tree
show_tree() {
    print_header "📁 Documentation Structure"
    echo ""

    if command -v tree &> /dev/null; then
        tree -C "$DOCS_DIR"
    else
        find "$DOCS_DIR" -type f -name "*.md" | sort | sed "s|$DOCS_DIR|docs|" | while read -r file; do
            echo "$file"
        done
    fi
}

# Main script logic
main() {
    if [[ $# -eq 0 ]]; then
        show_menu
        exit 0
    fi

    local command="$1"
    shift

    case "$command" in
        quick)
            show_quick_reference
            ;;
        endpoints)
            list_endpoints
            ;;
        mqtt)
            list_mqtt_topics
            ;;
        security)
            show_security
            ;;
        auth)
            show_auth_guide
            ;;
        webauthn)
            show_webauthn_guide
            ;;
        contracts)
            show_contracts
            ;;
        flows)
            show_flows
            ;;
        agents|context|notes|decision|mfa)
            show_category_endpoints "$command"
            ;;
        view)
            if [[ $# -eq 0 ]]; then
                print_error "Usage: $0 view <file>"
                exit 1
            fi
            view_doc "$1"
            ;;
        search)
            if [[ $# -eq 0 ]]; then
                print_error "Usage: $0 search <term>"
                exit 1
            fi
            search_docs "$1"
            ;;
        tree)
            show_tree
            ;;
        help|--help|-h)
            show_menu
            ;;
        *)
            print_error "Unknown command: $command"
            echo ""
            show_menu
            exit 1
            ;;
    esac
}

main "$@"
