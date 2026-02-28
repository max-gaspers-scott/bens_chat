import os
import httpx
# from openai import OpenAI
# from dotenv import load_dotenv
from typing import Dict
from fastapi import FastAPI
# from typing import List
# from langchain.document_loaders import DirectoryLoader
# from langchain.schema import Document

# DATA_PATH = "data"

# # Load environment variables from .env file
# load_dotenv()

# def load_documents() -> List[Document]:
#     loader = DirectoryLoader(DATA_PATH, glob="*.pdf")
#     documents = loader.load()
#     return documents


app = FastAPI()
@app.get("/health")
async def health_check() -> Dict[str, str]:
    return {"status": "healthy"}

@app.get("/chat")
async def chat() -> Dict[str, str]:
    return {"res": "working api connection"}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8003)
