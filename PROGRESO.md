# Bitacora de progreso

## Setup inicial

- Identidad git verificada: `user.name=Angel Sanabria`, `user.email=as1945228@gmail.com`. Correcto, se usa tal cual.
- `git flow` (la extension AVH) no esta instalada en el sistema. Se emula el modelo Gitflow a mano: rama `main` (produccion/entregable), rama `develop` (integracion), ramas `feature/<nombre>` por fase o avance, merge a `develop` y luego a `main` cuando corresponda. Si mas adelante se quiere instalar la extension real, no cambia el flujo, solo automatiza los mismos comandos.
- Hook `.git/hooks/commit-msg` creado y ejecutable: rechaza commits que contengan "co-authored-by", "claude", "anthropic" o "generated with" (sin distinguir mayusculas).
- Remote `origin` ya configurado por el usuario (SSH, github.com/asanabria-2021067/proyecto2-graficas).

## Pendiente

Esperando autorizacion para arrancar Fase 1 (base del proyecto Rust, camara orbital, render paralelo).
