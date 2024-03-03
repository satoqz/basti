#!/usr/bin/env python3

import click
import os
import subprocess
import tomllib
import sys

SERVICES = ["bastid", "etcd"]

SOURCE_DIR = os.path.dirname(os.path.realpath(__file__))
INVENTORY_PATH = f"{SOURCE_DIR}/inventory.toml"

BASTID_DOCKERFILE = f"{SOURCE_DIR}/docker/bastid.Dockerfile"
BASTID_IMAGE_TAG = "toasterwaver/bastid:latest"
BASTID_PORT = 1337
BASTID_WORKERS = 3

ETCD_DOCKERFILE = f"{SOURCE_DIR}/docker/etcd.Dockerfile"
ETCD_IMAGE_TAG = "toasterwaver/etcd:latest"
ETCD_CLUSTER_TOKEN = "basti-etcd-cluster"
ETCD_CLIENT_PORT = 2379
ETCD_PEER_PORT = 2380

with open(INVENTORY_PATH, "rb") as f:
    INVENTORY: dict[str, str | dict[str, str]] = tomllib.load(f)


@click.group()
def cli() -> None:
    pass


@cli.command(name="link")
def link_cmd() -> None:
    target = f"{os.environ["HOME"]}/.cargo/bin/x"
    os.remove(target)
    os.symlink(__file__, target)
    click.echo(f"Linked x.py to {target}.")


@cli.command(name="build")
@click.argument(
    "service",
    type=click.Choice(SERVICES),
    required=True,
)
def build_cmd(service: str) -> None:
    image_tag, dockerfile = {
        "bastid": (BASTID_IMAGE_TAG, BASTID_DOCKERFILE),
        "etcd": (ETCD_IMAGE_TAG, ETCD_DOCKERFILE),
    }[service]

    click.echo(f"building image {image_tag}...")
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
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.option("--group", "-g", type=str, default=INVENTORY["default_group"])
@click.option("--no-build", is_flag=True, type=bool, default=False)
@click.pass_context
def deploy_cmd(ctx: click.Context, service: str, group: str, no_build: bool) -> None:
    if not no_build:
        ctx.invoke(build_cmd, service=service)

    image_tag = {"bastid": BASTID_IMAGE_TAG, "etcd": ETCD_IMAGE_TAG}[service]

    for node in INVENTORY[group]:
        ssh = INVENTORY[group][node]["ssh"]

        result = subprocess.run(
            f"docker save '{image_tag}' | ssh '{ssh}' 'sudo docker load'",
            shell=True,
            stdin=False,
            capture_output=True,
        )

        if result.returncode == 0:
            click.echo(f"copied image {image_tag} to {node}.")
        else:
            click.echo(f"failed copying image to {node}:")
            click.echo(result.stderr)
            sys.exit(1)


@cli.command(name="stop")
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.option("--group", "-g", type=str, default=INVENTORY["default_group"])
def stop_cmd(service: str, group: str) -> None:
    for node in INVENTORY[group]:
        node_details = INVENTORY[group][node]
        result = subprocess.run(
            ["ssh", node_details["ssh"], f"sudo docker kill basti-{service}"],
            stdin=False,
            capture_output=True,
        )
        if result.returncode == 0:
            click.echo(f"stopped {service} on {node}.")


@cli.command(name="start")
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.option("--group", "-g", type=str, default=INVENTORY["default_group"])
@click.option("--deploy", is_flag=True, type=bool, default=False)
@click.option("--no-build", is_flag=True, type=bool, default=False)
@click.option("--no-stop", is_flag=True, type=bool, default=False)
@click.pass_context
def start_cmd(
    ctx: click.Context,
    service: str,
    group: str,
    deploy: bool,
    no_build: bool,
    no_stop: bool,
) -> None:
    if deploy:
        ctx.invoke(deploy_cmd, service=service, group=group, no_build=no_build)
    if not no_stop:
        ctx.invoke(stop_cmd, service=service, group=group)

    container_image = {
        "bastid": BASTID_IMAGE_TAG,
        "etcd": ETCD_IMAGE_TAG,
    }[service]

    container_ports = {
        "bastid": [BASTID_PORT],
        "etcd": [ETCD_CLIENT_PORT, ETCD_PEER_PORT],
    }[service]

    etcd_cluster_string = {
        "bastid": lambda: ",".join(
            f"http://{INVENTORY[group][node]["ip"]}:{ETCD_CLIENT_PORT}"
            for node in INVENTORY[group]
        ),
        "etcd": lambda: ",".join(
            f"{node}=http://{INVENTORY[group][node]["ip"]}:{ETCD_PEER_PORT}"
            for node in INVENTORY[group]
        ),
    }[service]()

    for node in INVENTORY[group]:
        node_details = INVENTORY[group][node]

        docker_command = (
            [
                "sudo",
                "docker",
                "run",
                "-d",
                "--init",
                "--rm",
                f"--name=basti-{service}",
            ]
            + [f"-p={port}:{port}" for port in container_ports]
            + [container_image, service]
        )

        service_args = {
            "bastid": lambda: [
                f"--workers={BASTID_WORKERS}",
                f"--listen=0.0.0.0:{BASTID_PORT}",
                f"--etcd={etcd_cluster_string}",
                f"--name={node}",
            ],
            "etcd": lambda: [
                f"--name={node}",
                f"--initial-advertise-peer-urls=http://{node_details["ip"]}:{ETCD_PEER_PORT}",
                f"--listen-peer-urls=http://0.0.0.0:{ETCD_PEER_PORT}",
                f"--listen-client-urls=http://0.0.0.0:{ETCD_CLIENT_PORT}",
                f"--advertise-client-urls=http://{node_details["ip"]}:{ETCD_CLIENT_PORT}",
                f"--initial-cluster-token={ETCD_CLUSTER_TOKEN}",
                f"--initial-cluster={etcd_cluster_string}",
                "--initial-cluster-state=new",
            ],
        }[service]()

        result = subprocess.run(
            ["ssh", node_details["ssh"], " ".join(docker_command + service_args)],
            stdin=False,
            capture_output=True,
        )

        if result.returncode == 0:
            click.echo(f"started {service} on {node}.")
        else:
            click.echo(f"failed starting {service} on {node}:")
            click.echo(result.stderr)
            sys.exit(1)


@cli.command(name="logs")
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.argument("node", type=str, required=True)
@click.option("--group", "-g", type=str, default=INVENTORY["default_group"])
@click.option("--follow", "-f", is_flag=True, type=bool, default=False)
def logs_cmd(service: str, node: str, group: str, follow: bool) -> None:
    docker_command = [
        "sudo",
        "docker",
        "logs",
        *(["-f"] if follow else []),
        f"basti-{service}",
    ]
    subprocess.run(
        ["ssh", INVENTORY[group][node]["ssh"], " ".join(docker_command)],
        stdin=False,
    )


@cli.command(name="ssh")
@click.argument("node", type=str, required=True)
@click.argument("args", type=str, nargs=-1)
@click.option("--group", "-g", type=str, default=INVENTORY["default_group"])
def ssh_cmd(node: str, args: list[str], group: str) -> None:
    subprocess.run(["ssh", "-t", INVENTORY[group][node]["ssh"], " ".join(args)])


if __name__ == "__main__":
    cli()
