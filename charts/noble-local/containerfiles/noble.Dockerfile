# FIXME - easier to just install nobled?
FROM alpine:3.14 AS base

RUN apk add --no-cache sed

FROM ghcr.io/noble-assets/noble:v8.0.0-rc.2 AS noble
#FROM ghcr.io/astriaorg/noble:astria AS noble

FROM base

# copy Noble's files from the noble image
COPY --from=noble /bin/nobled /bin/nobled
COPY --from=noble /bin/jq /bin/jq

CMD ["echo", "this is a dev image wrapping noble's image."]
