# GitHub Metrics

运行：

```shell
# 注意修改 GITHUB_USERNAME、GITHUB_ACCESS_TOKEN、GITHUB_REPOS。
docker run -it --rm -e GITHUB_USERNAME=YOUR_USERNAME -e GITHUB_ACCESS_TOKEN=YOUR_ACCESS_TOKEN -e GITHUB_REPOS=hamflx/huawei-pc-manager-bootstrap:hamflx/forward-dll -p 8080:8080 hamflx/github-metrics
```
