---
name: docker
description: Inspect and run containers with docker or podman. Use for images, containers, logs, compose, ports, or disk usage.
---

# docker

- `docker ps -a`, `docker images`, `docker stats --no-stream`.
- Logs: `docker logs --tail 100 <id>`; exec: `docker exec -it <id> sh`.
- Compose: `docker compose ps`, `docker compose logs -f svc`.
- Disk: `docker system df`. Ask before `docker system prune`.
- Never remove volumes unless asked; that is data.
