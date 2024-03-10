#!/usr/bin/env python3

from os import path

import click
import masoud

PROJECT_ROOT = path.dirname(__file__)


class Bastid(masoud.Service):
    name = "bastid"
    container_name = "basti-bastid"
    image = "satoqz.net/bastid:latest"
    dockerfile = (path.join(PROJECT_ROOT, "docker/bastid.Dockerfile"), PROJECT_ROOT)

    DEFAULT_PORT = 1337

    @property
    def ports(self) -> list[tuple[int, int]]:
        port = self.host.get_var("bastid_port", int) or self.DEFAULT_PORT
        return [(port, self.DEFAULT_PORT)]

    @property
    def command(self) -> list[str]:
        return [
            "bastid",
            f"--name={self.host.name}",
            f"--workers={self.host.get_var("bastid_workers", int) or 1}",
            f"--etcd=http://{self.host.must_get_var("ip", str)}:{self.host.get_var("etcd_client_port", int) or Etcd.DEFAULT_CLIENT_PORT}",
            f"--listen=0.0.0.0:{self.DEFAULT_PORT}",
        ]


class Etcd(masoud.Service):
    name = "etcd"
    container_name = "basti-etcd"
    image = "satoqz.net/etcd:latest"
    dockerfile = (path.join(PROJECT_ROOT, "docker/etcd.Dockerfile"), PROJECT_ROOT)

    DEFAULT_CLIENT_PORT = 2379
    DEFAULT_PEER_PORT = 2380

    @property
    def ports(self) -> list[tuple[int, int]]:
        return [
            (
                self.host.get_var("etcd_client_port", int) or self.DEFAULT_CLIENT_PORT,
                self.DEFAULT_CLIENT_PORT,
            ),
            (
                self.host.get_var("etcd_peer_port", int) or self.DEFAULT_PEER_PORT,
                self.DEFAULT_PEER_PORT,
            ),
        ]

    VOLUME_MOUNT = "/data"

    @property
    def volumes(self) -> list[tuple[str, str]]:
        return [
            (
                self.host.get_var("etcd_volume", str) or "basti-etcd-data",
                self.VOLUME_MOUNT,
            )
        ]

    @property
    def command(self) -> list[str]:
        ip = self.host.must_get_var("ip", str)
        client_port, peer_port = (
            self.host.get_var("etcd_client_port", int) or self.DEFAULT_CLIENT_PORT,
            self.host.get_var("etcd_peer_port", int) or self.DEFAULT_PEER_PORT,
        )
        initial_cluster = ",".join(
            f"{host.name}=http://{host.must_get_var("ip", str)}:{host.get_var("etcd_peer_port") or self.DEFAULT_PEER_PORT}"
            for host in self.group.get_hosts()
        )
        return [
            "etcd",
            f"--name={self.host.name}",
            f"--data-dir={self.VOLUME_MOUNT}/data",
            f"--wal-dir={self.VOLUME_MOUNT}/wal",
            f"--initial-advertise-peer-urls=http://{ip}:{peer_port}",
            f"--listen-peer-urls=http://0.0.0.0:{self.DEFAULT_PEER_PORT}",
            f"--listen-client-urls=http://0.0.0.0:{self.DEFAULT_CLIENT_PORT}",
            f"--advertise-client-urls=http://{ip}:{client_port}",
            f"--initial-cluster={initial_cluster}",
            "--initial-cluster-token=bastid-etcd-cluster",
            "--initial-cluster-state=new",
        ]


@masoud.cli.command
@click.pass_context
def print_cluster(ctx: click.Context):
    group: masoud.Group = ctx.obj["group"]
    click.echo(f"export BASTI_CLUSTER='{",".join([
        f"http://{host.must_get_var("ip", str)}:{host.get_var("bastid_port", int) or Bastid.DEFAULT_PORT}"
        for host in group.get_hosts()
    ])}'")


if __name__ == "__main__":
    masoud.cli(services=[Etcd, Bastid])
