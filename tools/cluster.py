#!/usr/bin/env python3

import click
import os
import subprocess
import tomllib
import sys
from typing import TypeVar

SERVICES = ["bastid", "etcd"]

TOOLS_DIR = os.path.dirname(os.path.realpath(__file__))
SOURCE_DIR = os.path.join(TOOLS_DIR, "..")
DOCKER_DIR = os.path.join(TOOLS_DIR, "../docker")
DEFAULT_INVENTORY_PATH = f"{TOOLS_DIR}/inventory.toml"

BASTID_DOCKERFILE = f"{DOCKER_DIR}/bastid.Dockerfile"
BASTID_IMAGE_TAG = "satoqz.net/bastid:latest"
BASTID_DEFAULT_PORT = 1337
BASTID_DEFAULT_WORKERS = 3

ETCD_DOCKERFILE = f"{DOCKER_DIR}/etcd.Dockerfile"
ETCD_IMAGE_TAG = "satoqz.net/etcd:latest"
ETCD_CLUSTER_TOKEN = "basti-etcd-cluster"
ETCD_VOLUME_MOUNT = "/data"
ETCD_DEFAULT_VOLUME_NAME = "basti-etcd-data"
ETCD_DEFAULT_CLIENT_PORT = 2379
ETCD_DEFAULT_PEER_PORT = 2380


class Node:
    name: str
    ssh: str
    ip: str
    bastid_port: int
    bastid_workers: int
    etcd_volume_name: str
    etcd_client_port: int
    etcd_peer_port: int

    def __init__(self, name: str, inventory: dict[str], group: dict[str]) -> None:
        raw_node: dict[str] = group[name]

        self.name = name
        self.ssh = raw_node["ssh"]
        self.ip = raw_node["ip"]

        T = TypeVar("T")

        def try_upwards(key: str, default: T) -> T:
            return (
                raw_node.get(key, None)
                or group.get(key, None)
                or inventory.get(key, default)
            )

        self.bastid_port = try_upwards("bastid_port", BASTID_DEFAULT_PORT)
        self.bastid_workers = try_upwards("bastid_workers", BASTID_DEFAULT_WORKERS)
        self.etcd_volume_name = try_upwards(
            "etcd_volume_name", ETCD_DEFAULT_VOLUME_NAME
        )
        self.etcd_client_port = try_upwards(
            "etcd_client_port", ETCD_DEFAULT_CLIENT_PORT
        )
        self.etcd_peer_port = try_upwards("etcd_peer_port", ETCD_DEFAULT_PEER_PORT)
        self.etcd_volume_name = try_upwards(
            "etcd_volume_name", ETCD_DEFAULT_VOLUME_NAME
        )


class Inventory:
    _path: str
    _group: str
    _nodes: dict[str, Node]

    def __init__(self, path: str, group: str | None) -> None:
        self._path = path
        with open(self._path, "rb") as f:
            raw_inventory = tomllib.load(f)
        self._group = group or raw_inventory["default_group"]
        raw_group = raw_inventory[self._group]
        self._nodes = {
            name: Node(name, raw_inventory, raw_group)
            for name in raw_group
            if isinstance(raw_group[name], dict)
        }

    def node(self, name: str) -> Node:
        if name not in self._nodes:
            raise ValueError(
                f"node {name} does not exist in group {self._group} of {self._path}"
            )
        return self._nodes[name]

    def nodes(self, *filter: list[str]) -> list[Node]:
        if not filter:
            return self._nodes.values()
        return [self._nodes[name] for name in self._nodes if name in filter]


pass_inventory = click.make_pass_decorator(Inventory)


@click.group()
@click.option("--inventory", "-i", type=str, default=DEFAULT_INVENTORY_PATH)
@click.option("--group", "-g", type=str, default=None)
@click.pass_context
def cli(ctx: click.Context, inventory: str, group: str | None) -> None:
    ctx.obj = Inventory(inventory, group)


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
    subprocess.run(
        ["docker", "build", "-t", image_tag, "-f", dockerfile, SOURCE_DIR],
        check=True,
    )


@cli.command(name="deploy")
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.argument("nodes", type=str, nargs=-1)
@click.option("--no-build", is_flag=True, type=bool, default=False)
@click.pass_context
def deploy_cmd(
    ctx: click.Context,
    service: str,
    nodes: list[str],
    no_build: bool,
) -> None:
    if not no_build:
        ctx.invoke(build_cmd, service=service)

    inventory: Inventory = ctx.obj

    image_tag = {"bastid": BASTID_IMAGE_TAG, "etcd": ETCD_IMAGE_TAG}[service]

    for node in inventory.nodes(*nodes):
        result = subprocess.run(
            f"docker save '{image_tag}' | ssh '{node.ssh}' 'sudo docker load'",
            shell=True,
            stdin=False,
            capture_output=True,
        )

        if result.returncode == 0:
            click.echo(f"copied image {image_tag} to {node.name}.")
        else:
            click.echo(f"failed copying image to {node.name}:")
            click.echo(result.stderr)
            sys.exit(1)


@cli.command(name="stop")
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.argument("nodes", type=str, nargs=-1)
@pass_inventory
def stop_cmd(inventory: Inventory, service: str, nodes: list[str]) -> None:
    for node in inventory.nodes(*nodes):
        result = subprocess.run(
            [
                "ssh",
                node.ssh,
                f"sudo docker kill basti-{service} && sudo docker rm basti-{service}",
            ],
            stdin=False,
            capture_output=True,
        )
        if result.returncode == 0:
            click.echo(f"stopped {service} on {node.name}.")


@cli.command(name="start")
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.argument("nodes", type=str, nargs=-1)
@click.option("--deploy", is_flag=True, type=bool, default=False)
@click.option("--no-build", is_flag=True, type=bool, default=False)
@click.option("--no-stop", is_flag=True, type=bool, default=False)
@click.pass_context
def start_cmd(
    ctx: click.Context,
    service: str,
    nodes: list[str],
    deploy: bool,
    no_build: bool,
    no_stop: bool,
) -> None:
    if deploy:
        ctx.invoke(deploy_cmd, service=service, nodes=nodes, no_build=no_build)
    if not no_stop:
        ctx.invoke(stop_cmd, service=service, nodes=nodes)

    inventory: Inventory = ctx.obj

    container_image = {
        "bastid": BASTID_IMAGE_TAG,
        "etcd": ETCD_IMAGE_TAG,
    }[service]

    etcd_cluster_string = ",".join(
        f"{node.name}=http://{node.ip}:{node.etcd_peer_port}"
        for node in inventory.nodes()
    )

    for node in inventory.nodes(*nodes):
        container_ports, container_volumes = {
            "bastid": ([(node.bastid_port, BASTID_DEFAULT_PORT)], []),
            "etcd": (
                [
                    (node.etcd_client_port, ETCD_DEFAULT_CLIENT_PORT),
                    (node.etcd_peer_port, ETCD_DEFAULT_PEER_PORT),
                ],
                [(ETCD_DEFAULT_VOLUME_NAME, ETCD_VOLUME_MOUNT)],
            ),
        }[service]

        docker_command = (
            [
                "sudo",
                "docker",
                "run",
                "-d",
                "--init",
                "--restart=unless-stopped",
                f"--name=basti-{service}",
            ]
            + [
                f"-p={outer_port}:{inner_port}"
                for outer_port, inner_port in container_ports
            ]
            + [f"-v={volume}:{mount}" for volume, mount in container_volumes]
            + [container_image, service]
        )

        service_args = {
            "bastid": lambda: [
                f"--workers={node.bastid_workers}",
                f"--listen=0.0.0.0:{BASTID_DEFAULT_PORT}",
                f"--etcd=http://{node.ip}:{node.etcd_client_port}",
                f"--name={node.name}",
            ],
            "etcd": lambda: [
                f"--name={node.name}",
                f"--data-dir={ETCD_VOLUME_MOUNT}/data",
                f"--wal-dir={ETCD_VOLUME_MOUNT}/wal",
                f"--initial-advertise-peer-urls=http://{node.ip}:{ETCD_DEFAULT_PEER_PORT}",
                f"--listen-peer-urls=http://0.0.0.0:{ETCD_DEFAULT_PEER_PORT}",
                f"--listen-client-urls=http://0.0.0.0:{ETCD_DEFAULT_CLIENT_PORT}",
                f"--advertise-client-urls=http://{node.ip}:{node.etcd_client_port}",
                f"--initial-cluster-token={ETCD_CLUSTER_TOKEN}",
                f"--initial-cluster={etcd_cluster_string}",
                "--initial-cluster-state=new",
            ],
        }[service]()

        result = subprocess.run(
            ["ssh", node.ssh, " ".join(docker_command + service_args)],
            stdin=False,
            capture_output=True,
        )

        if result.returncode == 0:
            click.echo(f"started {service} on {node.name}.")
        else:
            click.echo(f"failed starting {service} on {node.name}:")
            click.echo(result.stderr)
            sys.exit(1)


@cli.command(name="logs")
@click.argument("service", type=click.Choice(SERVICES), required=True)
@click.argument("node", type=str, required=True)
@click.option("--follow", "-f", is_flag=True, type=bool, default=False)
@pass_inventory
def logs_cmd(inventory: Inventory, service: str, node: str, follow: bool) -> None:
    docker_command = [
        "sudo",
        "docker",
        "logs",
        *(["-f"] if follow else []),
        f"basti-{service}",
    ]
    subprocess.run(
        ["ssh", inventory.node(node).ssh, " ".join(docker_command)],
        stdin=False,
    )


@cli.command(name="ssh")
@click.argument("node", type=str, required=True)
@click.argument("args", type=str, nargs=-1)
@pass_inventory
def ssh_cmd(inventory: Inventory, node: str, args: list[str]) -> None:
    subprocess.run(
        [
            "ssh",
            "-t",
            "--",
            inventory.node(node).ssh,
            "sudo",
            *(["-i"] if len(args) < 1 else []),
            *args,
        ]
    )


if __name__ == "__main__":
    cli()
