# stream-catch

This repository is organized as a Cargo workspace with shared infrastructure code in `crates/infra_postgres` and binaries in `backend` and `worker`.

The backend API service is deployed to Fly.io. The worker service will be deployed to DigitalOcean once its implementation is completed.
