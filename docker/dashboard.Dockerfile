# Symbion PWA Dashboard - Multi-stage Docker build
# Usage: docker build -f docker/dashboard.Dockerfile -t symbion-dashboard .

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM node:20-alpine AS builder

WORKDIR /build

COPY pwa-dashboard/package.json pwa-dashboard/package-lock.json ./

RUN npm ci

COPY pwa-dashboard/ .

RUN npm run build

# ============================================================================
# Stage 2: Serve with nginx
# ============================================================================
FROM nginx:alpine

COPY docker/nginx-dashboard.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /build/dist /usr/share/nginx/html

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD wget -qO- http://localhost:3000/ || exit 1
