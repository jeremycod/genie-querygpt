#!/bin/bash

# Load environment variables
source .env

# Run migrations
psql "$DATABASE_URL" -f migrations/001_create_materialized_views.sql

echo "Migrations completed"