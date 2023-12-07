import uvicorn

from open_edc.main import app  #  pragma: no cover

if __name__ == "__main__":
    uvicorn.run(app)
