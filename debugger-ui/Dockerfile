# Build stage prepares dependencies and bundles the SPA assets
FROM node:20-alpine AS build
WORKDIR /app

COPY package.json package-lock.json ./
RUN apk add --no-cache git
RUN npm install

COPY . .
RUN npm run build

# Runtime stage keeps the final image lean
FROM node:20-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV PORT=8080
ENV DEBUGGER_WS_PATH=/ws

# Copy everything we need from the build stage
COPY --from=build /app/package.json /app/package-lock.json ./
RUN apk add --no-cache git
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/dist ./dist
COPY --from=build /app/server ./server
COPY --from=build /app/src ./src
COPY --from=build /app/tsconfig.server.json ./

EXPOSE 8080

CMD ["npm", "run", "mock:start"]
