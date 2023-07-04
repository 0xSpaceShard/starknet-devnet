FROM ubuntu:22.04

RUN apt update && apt upgrade -y

# install packages to the container
RUN apt install -y gcc curl wget git vim zip sqlite3 libdigest-sha-perl zsh build-essential make

RUN apt update
RUN apt -y install pkg-config libssl-dev

# set variable
ARG USER=local_dev

# create user
RUN adduser \
   --system \
   --shell /bin/bash \
   --gecos $USER \
   --group \
   --disabled-password \
   --home /home/$USER \
   $USER


# # Set up Rust
USER $USER

# copy local bashrc to user home directory
COPY ./bashrc /home/$USER/.bashrc

# install rust from website
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y --no-modify-path

# apply bashrc file
SHELL ["/bin/bash", "-c"]
RUN source $HOME/.bashrc

USER root

EXPOSE 8000 8080