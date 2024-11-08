FROM rust:latest as builder

WORKDIR /usr/src/dusty-bot

COPY . .

RUN cargo install --path .