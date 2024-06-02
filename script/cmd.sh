# 生成yaml
goctl kube deploy -name redis -namespace adhoc -image redis:6-alpine -o redis.yaml -port 6379
