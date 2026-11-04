package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"net/url"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/minio/minio-go/v7"
	"github.com/minio/minio-go/v7/pkg/credentials"
)

type object struct {
	Key  string `json:"key"`
	Size int64  `json:"size"`
}

func run() error {
	if len(os.Args) == 2 && os.Args[1] == "version" {
		fmt.Println("ontoforge-bucket DEVELOPMENT minio-go/v7 v7.0.91 source 9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a")
		return nil
	}
	if len(os.Args) < 2 || (os.Args[1] != "make" && os.Args[1] != "list") {
		return errors.New("expected make or list")
	}
	flags := flag.NewFlagSet("bucket", flag.ContinueOnError)
	flags.SetOutput(io.Discard)
	bucket := flags.String("bucket", "", "bucket name")
	if err := flags.Parse(os.Args[2:]); err != nil || *bucket == "" || flags.NArg() != 0 {
		return errors.New("invalid bucket arguments")
	}
	endpoint, err := url.Parse(os.Getenv("S3_ENDPOINT"))
	if err != nil || endpoint.Host == "" || endpoint.User != nil ||
		(endpoint.Scheme != "http" && endpoint.Scheme != "https") ||
		(endpoint.Path != "" && endpoint.Path != "/") || endpoint.RawQuery != "" || endpoint.Fragment != "" {
		return errors.New("invalid S3 endpoint")
	}
	access, secret := os.Getenv("AWS_ACCESS_KEY_ID"), os.Getenv("AWS_SECRET_ACCESS_KEY")
	if strings.TrimSpace(access) == "" || strings.TrimSpace(secret) == "" {
		return errors.New("missing S3 credentials")
	}
	seconds := 30
	if value := os.Getenv("BUCKET_DEADLINE_SECONDS"); value != "" {
		seconds, err = strconv.Atoi(value)
		if err != nil || seconds < 1 || seconds > 60 {
			return errors.New("deadline must be between 1 and 60 seconds")
		}
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Duration(seconds)*time.Second)
	defer cancel()
	client, err := minio.New(endpoint.Host, &minio.Options{
		Creds: credentials.NewStaticV4(access, secret, ""), Secure: endpoint.Scheme == "https", Region: "us-east-1",
	})
	if err != nil {
		return err
	}
	if os.Args[1] == "make" {
		return client.MakeBucket(ctx, *bucket, minio.MakeBucketOptions{Region: "us-east-1"})
	}
	objects := make([]object, 0)
	for entry := range client.ListObjects(ctx, *bucket, minio.ListObjectsOptions{Recursive: true}) {
		if entry.Err != nil {
			return entry.Err
		}
		objects = append(objects, object{Key: entry.Key, Size: entry.Size})
	}
	if err := ctx.Err(); err != nil {
		return err
	}
	return json.NewEncoder(os.Stdout).Encode(objects)
}

func main() {
	if err := run(); err != nil {
		message := err.Error()
		for _, name := range []string{"AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"} {
			if value := os.Getenv(name); value != "" {
				message = strings.ReplaceAll(message, value, "[redacted]")
			}
		}
		fmt.Fprintln(os.Stderr, message)
		os.Exit(1)
	}
}
