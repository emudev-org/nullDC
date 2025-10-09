# Build stage prepares dependencies and bundles the SPA assets
FROM node:20-alpine AS build
WORKDIR /app

# Install build tools and Rust for WASM builds
RUN apk add --no-cache bash curl build-base git openssl-dev

# Install Rust (minimal profile, no extra docs)
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# Add wasm32 target for Rust
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-pack --locked

# Install dependencies
COPY package.json package-lock.json ./
RUN npm install

# Copy source and build
COPY . .
RUN npm run build

# Runtime stage keeps the final image lean
FROM node:20-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV PORT=8080
ENV DEBUGGER_WS_PATH=/ws

# Copy runtime dependencies only
COPY --from=build /app/package.json /app/package-lock.json ./
RUN apk add --no-cache git
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/dist ./dist
COPY --from=build /app/server ./server
COPY --from=build /app/src ./src
COPY --from=build /app/tsconfig.server.json ./

EXPOSE 8080
CMD ["npm", "run", "mock:start"]
