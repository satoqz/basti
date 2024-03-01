#!/usr/bin/env python3

import click
import subprocess
import tomllib
import sys


BASTID_DOCKERFILE = "bastid.Dockerfile"
BASTID_IMAGE_TAG = "toasterwaver/bastid:latest"
BASTID_CONTAINER_NAME = "basti-bastid"
BASTID_PORT = 1337

ETCD_DOCKERFILE = "etcd.Dockerfile"
ETCD_IMAGE_TAG = "toasterwaver/etcd:latest"
ETCD_CONTAINER_NAME = "basti-etcd"
ETCD_CLUSTER_TOKEN = "basti-etcd-cluster"
ETCD_CLIENT_PORT = 2379
ETCD_PEER_PORT = 2380

with open("inventory.toml", "rb") as f:
    INVENTORY: dict[str, str | dict[str, str]] = tomllib.load(f)


@click.group()
def cli() -> None:
    pass


@cli.command(name="build")
@click.argument(
    "service",
    type=click.Choice(["bastid", "etcd"]),
    required=True,
)
def build_cmd(service: str) -> None:
    image_tag, dockerfile = {
        "bastid": (BASTID_IMAGE_TAG, BASTID_DOCKERFILE),
        "etcd": (ETCD_IMAGE_TAG, ETCD_DOCKERFILE),
    }[service]

    result = subprocess.run(
        ["docker", "build", "-t", image_tag, "-f", dockerfile, "."],
        stdin=False,
        capture_output=True,
    )

    if result.returncode == 0:
        click.echo(f"built image {image_tag}.")
    else:
        click.echo("failed building image:")
        click.echo(result.stderr)
        sys.exit(1)


@cli.command(name="deploy")
@click.argument("service", type=click.Choice(["bastid", "etcd"]), required=True)
@click.option("--group", type=str, default=INVENTORY["default_group"])
@click.option("--build/--no-build", type=bool, default=True)
@click.pass_context
def deploy_cmd(ctx: click.Context, service: str, group: str, build: bool) -> None:
    if build:
        ctx.invoke(build_cmd, service=service)

    image_tag = {"bastid": BASTID_IMAGE_TAG, "etcd": ETCD_IMAGE_TAG}[service]

    for host in INVENTORY[group]:
        ssh = INVENTORY[group][host]["ssh"]

        result = subprocess.run(
            f"docker save '{image_tag}' | ssh '{ssh}' 'sudo docker load'",
            shell=True,
            stdin=False,
            capture_output=True,
        )

        if result.returncode == 0:
            click.echo(f"copied image {image_tag} to {host}.")
        else:
            click.echo(f"failed copying image to {host}:")
            click.echo(result.stderr)
            sys.exit(1)


@cli.command(name="stop")
@click.argument("service", type=click.Choice(["bastid", "etcd"]), required=True)
@click.option("--group", type=str, default=INVENTORY["default_group"])
def stop_cmd(service: str, group: str) -> None:
    container_name = {
        "bastid": BASTID_CONTAINER_NAME,
        "etcd": ETCD_CONTAINER_NAME,
    }[service]

    for host in INVENTORY[group]:
        host_details = INVENTORY[group][host]
        result = subprocess.run(
            f"ssh '{host_details["ssh"]}' 'sudo docker kill {container_name}'",
            shell=True,
            stdin=False,
            capture_output=True,
        )
        if result.returncode == 0:
            click.echo(f"stopped {service} on {host}.")


@cli.command(name="start")
@click.argument("service", type=click.Choice(["bastid", "etcd"]), required=True)
@click.option("--group", type=str, default=INVENTORY["default_group"])
@click.option("--deploy/--no-deploy", type=bool, default=False)
@click.option("--build/--no-build", type=bool, default=True)
@click.option("--stop/--no-stop", type=bool, default=True)
@click.pass_context
def start_cmd(
    ctx: click.Context,
    service: str,
    group: str,
    deploy: bool,
    build: bool,
    stop: bool,
) -> None:
    if stop:
        ctx.invoke(stop_cmd, service=service, group=group)
    if deploy:
        ctx.invoke(deploy_cmd, service=service, group=group, build=build)
    {"bastid": start_bastid, "etcd": start_etcd}[service](group)


def start_bastid(group: str) -> None:
    for host in INVENTORY[group]:
        host_details = INVENTORY[group][host]
        docker_command = " ".join(
            [
                "sudo",
                "docker",
                "run",
                "-d",
                "--init",
                "--rm",
                f"--name={BASTID_CONTAINER_NAME}",
                f"-p={BASTID_PORT}:{BASTID_PORT}",
                BASTID_IMAGE_TAG,
                "bastid",
            ]
        )
        result = subprocess.run(
            f"ssh '{host_details["ssh"]}' '{docker_command}'",
            shell=True,
            stdin=False,
            capture_output=True,
        )

        if result.returncode == 0:
            click.echo(f"started bastid on {host}.")
        else:
            click.echo(f"failed starting bastid on {host}:")
            click.echo(result.stderr)
            sys.exit(1)


def start_etcd(group: str) -> None:
    cluster = ",".join(
        f"{host}=http://{INVENTORY[group][host]["ip"]}:{ETCD_PEER_PORT}"
        for host in INVENTORY[group]
    )

    for host in INVENTORY[group]:
        host_details = INVENTORY[group][host]
        docker_command = " ".join(
            [
                "sudo",
                "docker",
                "run",
                "-d",
                "--init",
                "--rm",
                f"--name={ETCD_CONTAINER_NAME}",
                f"-p={ETCD_CLIENT_PORT}:{ETCD_CLIENT_PORT}",
                f"-p={ETCD_PEER_PORT}:{ETCD_PEER_PORT}",
                ETCD_IMAGE_TAG,
                "etcd",
                f"--name={host}",
                f"--initial-advertise-peer-urls=http://{host_details["ip"]}:{ETCD_PEER_PORT}",
                f"--listen-peer-urls=http://0.0.0.0:{ETCD_PEER_PORT}",
                f"--listen-client-urls=http://0.0.0.0:{ETCD_CLIENT_PORT}",
                f"--advertise-client-urls=http://{host_details["ip"]}:{ETCD_CLIENT_PORT}",
                f"--initial-cluster-token={ETCD_CLUSTER_TOKEN}",
                f"--initial-cluster={cluster}",
                "--initial-cluster-state=new",
            ]
        )
        result = subprocess.run(
            f"ssh '{host_details["ssh"]}' '{docker_command}'",
            shell=True,
            stdin=False,
            capture_output=True,
        )

        if result.returncode == 0:
            click.echo(f"started etcd on {host}.")
        else:
            click.echo(f"failed starting etcd on {host}:")
            click.echo(result.stderr)
            sys.exit(1)


if __name__ == "__main__":
    cli()
