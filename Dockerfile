FROM rust:latest

# Let's switch our working directory to `app` (equivalent to `cd app`)
# The `app` folder will be created for us by Docker in case it does not
# exist already.
WORKDIR /app
RUN apt update && apt install lld clang -y
# Copy all files from our working environment to our Docker image
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release
ENV APP_ENVIRONMENT=production
# When `docker run` is executed, launch the bianry
ENTRYPOINT [ "./target/release/zero2prod" ]
