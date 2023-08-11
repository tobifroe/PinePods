#!/bin/bash

# Read the API key from /tmp/web_api_key.txt
API_KEY=$(cat /tmp/web_api_key.txt)

# Call the FastAPI endpoint using the API key
curl "http://localhost:8032/api/data/refresh_pods?api_key=$API_KEY" >> /cron.log 2>&1