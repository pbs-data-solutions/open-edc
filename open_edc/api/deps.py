import logging

from open_edc.core.config import config

logging.basicConfig(format="%asctime)s - %(levelname)s - [%(filename)s:%(lineno)d] - %(message)s")
logging.root.setLevel(level=config.log_level)
logger = logging
