#!/bin/bash
set -e

# Check if DockerHub username is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <dockerhub-username>"
    echo "Example: $0 myusername"
    exit 1
fi

DOCKERHUB_USERNAME=$1

echo "🔨 Building and pushing images to DockerHub as: $DOCKERHUB_USERNAME"
echo ""

# Login to DockerHub
echo "🔐 Logging into DockerHub..."
docker login

# Build and push Rust backend app
echo ""
echo "🦀 Building bens_chat-app..."
docker build -t $DOCKERHUB_USERNAME/bens_chat-app:latest -t $DOCKERHUB_USERNAME/bens_chat-app:$(date +%Y%m%d) .

echo "📤 Pushing bens_chat-app..."
docker push $DOCKERHUB_USERNAME/bens_chat-app:latest
docker push $DOCKERHUB_USERNAME/bens_chat-app:$(date +%Y%m%d)

# Build and push Python FastAPI service
echo ""
echo "🐍 Building bens_chat-python..."
docker build -t $DOCKERHUB_USERNAME/bens_chat-python:latest -t $DOCKERHUB_USERNAME/bens_chat-python:$(date +%Y%m%d) ./fastapi-template

echo "📤 Pushing bens_chat-python..."
docker push $DOCKERHUB_USERNAME/bens_chat-python:latest
docker push $DOCKERHUB_USERNAME/bens_chat-python:$(date +%Y%m%d)

echo ""
echo "✅ All images pushed successfully!"
echo ""
echo "📋 Next steps:"
echo "1. Copy docker-compose.prod.yaml to your server"
echo "2. Create .env file on server with your credentials"
echo "3. Run: docker compose -f docker-compose.prod.yaml up -d"
