FROM rust:buster

WORKDIR /usr/src/develop
# COPY Cargo.toml .
# COPY src ./src

# RUN cargo install --path .

CMD ["/bin/bash"]
