+++
title = "Railway"
weight = 20
+++

Railway is a good fit for an Ansorum site repository when you want to deploy
the site content itself, not the `ansorum` source repository.

The normal Railway shape is:

- your repository contains `config.toml`, `content/`, `static/`, and related
  Ansorum site files
- Railway installs a pinned `ansorum` binary during the build
- Railway runs `ansorum serve` as the long-lived process

This guide assumes you are deploying that kind of site repository.

## Before You Start

Make sure your repository already builds locally with Ansorum:

```bash
ansorum build
ansorum serve
```

You will also want a real Ansorum release tag to install from, for example
`vX.Y.Z`.

## How Railway Sees This Repo

Railway uses Railpack to detect build and start behavior for most repositories.
Build and start commands can be overridden in the service settings when needed.

For an Ansorum site repository, you should override both commands explicitly so
Railway installs the `ansorum` binary and then starts the site server.

Important: if your site repository contains a root `Dockerfile`, Railway will
use that `Dockerfile` automatically instead of the normal Railpack path.
If you want the non-Docker workflow described here, remove or rename the root
`Dockerfile`, or switch the service builder back to Railpack.

## Deploy From GitHub

1. Create a new Railway project.
2. Choose **Deploy from GitHub repo**.
3. Select your Ansorum site repository.
4. Wait for Railway to create the service.

At this point, the service exists, but it still needs explicit build and start
commands.

## Build Command

Set the Railway **Build Command** to download a pinned Ansorum release asset
into the service image:

```sh
set -eux
VERSION=vX.Y.Z
TARGET=x86_64-unknown-linux-musl
mkdir -p /app/.ansorum/bin
curl -fsSL "https://github.com/nibzard/ansorum/releases/download/${VERSION}/ansorum-${VERSION}-${TARGET}.tar.gz" \
  | tar -xz -C /app/.ansorum/bin ansorum
/app/.ansorum/bin/ansorum --version
```

Replace `vX.Y.Z` with the release you want to pin to.

If the release tag you choose does not have uploaded binary assets yet, this
download step will fail. In that case, either pin to a tag that does have
assets or switch the service to a Docker/source-build workflow instead of the
release-asset download flow shown here.

The static `musl` target is the safest default on Railway because it avoids
glibc compatibility questions in the runtime image.

## Start Command

For the first deploy, the simplest Railway **Start Command** is:

```sh
/app/.ansorum/bin/ansorum serve --interface 0.0.0.0 --port "$PORT" --base-url /
```

This works well when you want root-relative URLs and do not need a fixed
canonical absolute URL yet.

Once you have generated a Railway domain, you can switch to:

```sh
/app/.ansorum/bin/ansorum serve --interface 0.0.0.0 --port "$PORT" --base-url "https://${RAILWAY_PUBLIC_DOMAIN}" --no-port-append
```

If you use a custom domain, prefer setting the explicit domain instead:

```sh
/app/.ansorum/bin/ansorum serve --interface 0.0.0.0 --port "$PORT" --base-url "https://docs.example.com" --no-port-append
```

## Generate a Public Domain

After the first deploy:

1. Open the Railway service.
2. Go to **Settings**.
3. Find **Networking -> Public Networking**.
4. Click **Generate Domain**.

Railway will assign a `*.up.railway.app` domain. That domain is also exposed to
the running service as `RAILWAY_PUBLIC_DOMAIN`.

If you bring your own domain, add it in the same networking area and then use
that custom domain in the Ansorum `--base-url` value.

## Why The Start Command Looks Like This

Ansorum must listen on the Railway-provided port, and Railway expects public
services to bind to `0.0.0.0:$PORT`.

That is why the command always includes:

- `--interface 0.0.0.0`
- `--port "$PORT"`

`ansorum serve` is the right long-running process because it builds the site and
then serves it from one command.

When `--base-url` is an absolute URL, add `--no-port-append` so Ansorum keeps
public URLs canonical instead of appending Railway's internal listen port.

## Recommended Production Shape

For a real production deployment:

- pin to an explicit Ansorum release tag in the Build Command
- use a custom domain instead of the generated Railway domain
- set `--base-url` to that custom domain
- keep the repository free of a root `Dockerfile` unless you intentionally want
  Railway to deploy through Docker instead of Railpack

## If You Prefer Config As Code

Railway also supports storing build and deploy configuration in a
`railway.toml` or `railway.json` file alongside your code. That can be useful
once you want the same Build Command and Start Command checked into the site
repository instead of configured only in the Railway dashboard.

## References

- Railway Build and Start Commands: https://docs.railway.com/builds/build-and-start-commands
- Railway Start Command behavior: https://docs.railway.com/deployments/start-command
- Railway Public Networking and `PORT`: https://docs.railway.com/public-networking
- Railway Variables reference for `RAILWAY_PUBLIC_DOMAIN`: https://docs.railway.com/reference/variables
- Railway Dockerfile detection: https://docs.railway.com/builds/dockerfiles
- Railway Config as Code: https://docs.railway.com/config-as-code/reference
