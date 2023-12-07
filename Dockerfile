FROM ubuntu:20.04

ENV LANG=C.UTF-8 \
    LC_ALL=zh_CN.utf8

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends language-pack-zh-han* curl iputils-ping supervisor && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /var/cache/apt/* && \
    localedef -c -f UTF-8 -i zh_CN zh_CN.UTF-8 && \
    ln -svf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime

COPY <file_name> .
COPY supervisord.conf /etc/supervisor/supervisord.conf

ENTRYPOINT ["supervisord", "-c", "/etc/supervisor/supervisord.conf", "--nodaemon"]

# docker build -t <contianer_name>:tls .
