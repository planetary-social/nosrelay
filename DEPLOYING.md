This service is deployed via ansible.

If you just need to update the nosrelay docker image running at news.nos.social you can:
1. Make some changes to the `news`
2. Wait for the [CI Pipeline](https://github.com/planetary-social/nosrelay/actions/workflows/ci.yml) to complete
3. Create a Github personal access token with package:write permissions
4. Authenticate with ghcr.io from the command line. Enter your personal access token when prompted for the password. `docker login ghcr.io -u USERNAME`
5. Pull down the sha of the built image: `docker pull --platform linux/amd64 ghcr.io/planetary-social/nosrelay@sha256:e4f6feebc007c71db595f6b682ed9b6fd7f234041c12a53193dfa3386dbcae2b`
6. Tag it with the `news` tag: `docker tag ghcr.io/planetary-social/nosrelay@sha256:e4f6feebc007c71db595f6b682ed9b6fd7f234041c12a53193dfa3386dbcae2b ghcr.io/planetary-social/nosrelay:news`
7. Push the tag: `docker push ghcr.io/planetary-social/nosrelay:news`
8. Wait for the image to be pulled by the server. You can watch by sshing into the server and running `sudo tail -f /var/log/syslog -n 100`