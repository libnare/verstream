# verstream

## Environment
`ADDRESS` - Server address (default: 127.0.0.1)
<br>
`PORT` - Server port (default: 8080)
<br>
`AWS_BUCKET` - S3 bucket name (required)
<br>
`AWS_REGION` - S3 region (optional)
<br>
`AWS_ENDPOINT` - S3 endpoint (optional)
<br>
`AWS_ACCESS_KEY_ID` - S3 Access key (optional)
<br>
`AWS_SECRET_ACCESS_KEY` S3 Secret key (optional)
<br>
`HEADER_CC_1Y` - Add header `Cache-Control: public, max-age=31536000` (optional)

## Docker Image
```docker
asia-docker.pkg.dev/libnare/verstream/main:latest
```