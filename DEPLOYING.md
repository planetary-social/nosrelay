This service is deployed via ansible.

If you just need to update the nosrelay docker image running at news.nos.social you can:
1. Make some changes to the `news` branch.
2. Wait for the [CI Pipeline](https://github.com/planetary-social/nosrelay/actions/workflows/ci.yml) to complete
3. Create a Github personal access token with package:write permissions
4. Authenticate with ghcr.io from the command line. Enter your personal access token when prompted for the password. `docker login ghcr.io -u USERNAME`
5. Find the [sha of the latest news image](https://github.com/planetary-social/nosrelay/pkgs/container/nosrelay/versions) and then run `./scripts/tag_as_news.sh SHA`
6. Wait for the image to be pulled by the server. You can watch by sshing into the server and running `sudo tail -f /var/log/syslog -n 100`