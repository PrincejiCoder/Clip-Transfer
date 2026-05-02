#!/bin/bash

# Check if wget is installed; if not, try to use curl
if ! command -v wget &> /dev/null
then
    download_command="curl -O"
else
    download_command="wget"
fi

# Get installation directory from user
echo -e "\033[1mEnter installation directory (default is /usr/share/linkdrop):\033[0m"
read install_dir
install_dir=${install_dir:-/usr/share/linkdrop}

# Create directory and download files
mkdir -p $install_dir
cd $install_dir
$download_command https://raw.githubusercontent.com/szabodanika/linkdrop/master/.env
$download_command https://raw.githubusercontent.com/szabodanika/linkdrop/master/compose.yaml

# Get public path URL and port from user
echo -e "\033[1mEnter public path URL (e.g. https://linkdrop.myserver.net or http://localhost:8080):\033[0m"
read public_path

echo -e "\033[1mEnter port number (default is 8080):\033[0m"
read port
port=${port:-8080}

# Update environment variables in .env file
sed -i "s|linkdrop_PUBLIC_PATH=.*|linkdrop_PUBLIC_PATH=${public_path}|" .env
sed -i "s|linkdrop_PORT=.*|linkdrop_PORT=${port}|" .env

# Start linkdrop using Docker Compose
docker compose --env-file .env up --detach
