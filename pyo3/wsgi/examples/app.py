import wsgiref.util

def application(environ, start_response):
    try:
        status = "201 Created"
        headers = [("Content-type", "text/plain")]
        wb = start_response(status, headers)
        wb("body unicode\r\n")
        wb(b"body bytes\r\n")

        yield "Environ ---------------------------------\r\n"
        yield f"Scheme  : {wsgiref.util.guess_scheme(environ)}\r\n"
        yield f"REQ URI : {wsgiref.util.request_uri(environ)}\r\n".encode("utf-8")
        yield f"APP URI : {wsgiref.util.application_uri(environ)}\r\n"
        yield ["-----------------------------------------\r\n", ]

        input = environ["wsgi.input"]
        for i in input:
            yield f"{i}\r\n"

        raise Exception("error occured.")
    except Exception as e:
        errors = environ["wsgi.errors"]
        errors.write(str(e))
