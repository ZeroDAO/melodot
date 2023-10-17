FROM docker.io/library/ubuntu:22.04

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		ca-certificates && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user and link ~/.local/share/melodot to /data
	useradd -m -u 1000 -U -s /bin/sh -d /melodot melodot && \
	mkdir -p /data /melodot/.local/share && \
	chown -R melodot:melodot /data && \
	ln -s /data /melodot/.local/share/melodot-node

USER melodot

# copy the compiled binary to the container
COPY --chown=melodot:melodot --chmod=774 melodot-node /usr/bin/melodot-node

# check if executable works in this container
RUN /usr/bin/melodot-node --version

# ws_port
EXPOSE 9930 9333 9944 30333 30334

CMD ["/usr/bin/melodot-node"]