package service

import "strings"

type Service struct{}

func (s Service) Run(name string) string {
	return strings.TrimSpace(name)
}

func (s *Service) Process(name string) string {
	return strings.ToUpper(name)
}

func ProcessRequest(value string) string {
	return strings.ToUpper(value)
}
