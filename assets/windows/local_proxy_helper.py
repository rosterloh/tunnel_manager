from argparse import ArgumentParser
import logging
from shutil import which
import subprocess

import boto3

'''
Helper module to run and execute a local proxy on Windows machines.
This must be in the dir as localproxy.exe with dependencies, inluding the cert dir.

Dependencies to install: 
python -m pip install boto3

Exactly one client is allowed to access the device remotely.

Usage:

options:
  -h, --help            show this help message and exit
  -c CREATE_DEVICE_TUNNEL, --create_device_tunnel CREATE_DEVICE_TUNNEL
                        Create a new tunnel for a <device>. Returned is both a source and a destination token
  -g GET_DEVICE_TUNNEL_STATUS, --get_device_tunnel_status GET_DEVICE_TUNNEL_STATUS
                        Get tunnel status for a <device> - either Open, Closed, or No tunnel
  -ro ROTATE_DEVICE_TOKENS, --rotate_device_tokens ROTATE_DEVICE_TOKENS
                        Rotate tokens for <device> returned a source and a destination token
  -con CONNECT_DEVICE, --connect_device CONNECT_DEVICE
                        Connect to a <device>. This starts the localproxy component service and creates or reconnects
                        to an existing tunnnel
  -p PROFILE, --profile PROFILE
                        Specify a valid AWS profile name
  -r REGION, --region REGION

IoT Tunnelling API:
https://boto3.amazonaws.com/v1/documentation/api/latest/reference/services/iotsecuretunneling.html
'''

logging.getLogger().setLevel(logging.INFO)

OPEN = {'OPEN'}
CLOSED = {'CLOSED'}
NO_TUNNEL = {'NO TUNNEL'}
SOURCE_TOKEN = "source"
DESTINATION_TOKEN = "destination"
CLIENT_MODES = {'S':'SOURCE','D':'DESTINATION', 'A':'ALL'}
GORT = 'GORT'
SSH = 'SSH'
SSH_PORT = 2222
GORT_PORT = 5555
SERVICES = [GORT,SSH]

client = None
REGION = None


class TunnelStatusException(Exception):
    def __init__(self,message):
        self.message = message
        super().__init__(self.message)


def start_boto3_client_session(profile, region):
    '''
        Get the boto3 client session which will be used to connect to AWS
        params - profile
    '''
    global client, REGION
    try:
        REGION = region
        session = boto3.Session(profile_name=profile, region_name=REGION)

        client = session.client('iotsecuretunneling')
    except Exception as e:
        logging.exception(f"Exception in starting the AWS boto3 session:{e}")


def _get_open_tunnel_for_device(deviceId):
    '''
        Get the status of the tunnel. If no tunnels are assign to the device, or there
        are more than one, raise and error.
        params - deviceId
    '''
    global client
    tunnel_summaries = client.list_tunnels(thingName=deviceId).get('tunnelSummaries',[])
    open_tunnels = [open_tunnel for open_tunnel in tunnel_summaries if open_tunnel.get('status') in OPEN]
    closed_tunnels = [closed_tunnel for closed_tunnel in tunnel_summaries if closed_tunnel.get('status') in CLOSED]
    
    if len(closed_tunnels) > 0:
        for closed_tunnel in closed_tunnels:
            delete_tunnel(closed_tunnel.get('tunnelId'))
    if len(open_tunnels) > 1:      
        raise TunnelStatusException("There are more than 1 tunnel created for this device. Only one OPEN tunnel is allowed")

    if len(open_tunnels) == 0:
        status = "NO TUNNEL"
        tunnel_id = None

    else:
        open_tunnel = open_tunnels[0]
        status = open_tunnel.get('status')
        tunnel_id = open_tunnel.get('tunnelId')

    return status, tunnel_id

def is_open(device_id):
    '''
        Given a specified device ID, when the application checks for an existing open tunnel,
        determine if an open tunnel already exists for the device.
    '''
    try:
        status,tunnel_id = _get_open_tunnel_for_device(device_id)
        logging.info(f'Tunnel with id {tunnel_id} for {device_id} is {status}.')
    except TunnelStatusException as e:
        logging.exception(str(e))

def rotate_access_tokens(device_id):
    '''
    Given an existing open tunnel for the specified device ID, when the application detects it,
    it should rotate the access tokens for both the source and destination clients before establishing a connection as the source client.
    '''
    try:
        global client
        status,tunnel_id = _get_open_tunnel_for_device(device_id)

        if status not in OPEN:
            logging.info(f'Cannot rotate tokens as tunnel with id {tunnel_id} for {device_id} is {status}. Create a new tunnel')
        else:
            destinationConfig={'thingName': device_id,'services': SERVICES}
            tokens = client.rotate_tunnel_access_token(tunnelId=tunnel_id,clientMode=CLIENT_MODES.get('A'),destinationConfig=destinationConfig)
            sourceAccessToken = tokens.get('sourceAccessToken')
            destinationAccessToken = tokens.get('destinationAccessToken')
            logging.info(f'Tokens rotated for tunnel with id {tunnel_id} for {device_id}')
            logging.debug(f'Source Token {sourceAccessToken}')
            logging.debug(f'Destination Token {destinationAccessToken}')
            return sourceAccessToken, destinationAccessToken

    except TunnelStatusException as e:
        logging.exception(str(e))
        return None


def open_tunnel(device_id):
    '''
    Given a request for a new token, when the application does not find an existing open tunnel for the specified device ID,
    it should correctly request a new tunnel to be opened.
    '''
    global client
    tunnel_summaries = client.list_tunnels(thingName=device_id).get('tunnelSummaries')

    if len(tunnel_summaries) == 0:
        logging.info(f"Create a new tunnel for {device_id}")

        tokens = client.open_tunnel(
            destinationConfig={
                'thingName': device_id,
                'services': SERVICES
            })
        tunnel_id = tokens.get('tunnelId')
        sourceAccessToken = tokens.get('sourceAccessToken')
        destinationAccessToken = tokens.get('destinationAccessToken')
        logging.info(f"New tunnel created {tunnel_id} for {device_id}")
        return sourceAccessToken, destinationAccessToken

    else:
        status = tunnel_summaries[0].get('status')
        tunnel_id = tunnel_summaries[0].get('tunnelId')
        logging.info(f"Not Opening a new tunnel. There is a tunnel {tunnel_id} for {device_id} with status {status}")
        return None, None

def delete_tunnel(tunnel_id):
    '''
    Delete the tunnel - this isn't exposed via command line args, but you can
    '''
    global client
    logging.info(f"Deleting tunnel {tunnel_id}")
    client.close_tunnel(tunnelId=tunnel_id,delete=True)


def start_local_proxy_for_source(device_id, ssh_port, gort_port):
    '''
    Run a  new local_proxy process - only one secure tunnel can be open per device.
    If another client runs lp, it will rotate the source and destination tokens
    and "kick off" any connected devices.
    '''
    status,tunnel_id=_get_open_tunnel_for_device(device_id)

    logging.info(f"Starting a new local proxy process for {device_id}")

    if status in NO_TUNNEL:
        logging.info(f"There is NO TUNNEL for device {device_id} - created a new local proxy process")
        source_token,_ = open_tunnel(device_id)
    elif status in OPEN:
        logging.info(f"Tunnel {tunnel_id} is OPEN for device {device_id}. Rotate Access Tokens")
        source_token,_ = rotate_access_tokens(device_id)
    else:
        logging.info(f"Tunnel {tunnel_id} is CLOSED for device {device_id}. delete and create a new Tunnel")
        delete_tunnel(tunnel_id)
        source_token,_ = open_tunnel(device_id)

    _run_lp_process(source_token, ssh_port, gort_port)

def _run_lp_process(source_token, ssh_port, gort_port):
    '''
    Run the local proxy for the GORT client
    '''
    global REGION
    # Check if we have a system version of localproxy to use
    lp = which('localproxy')
    if lp is None:
      lp = './localproxy'
    cmd = f'{lp} -r {REGION} -s {SSH}={ssh_port},{GORT}={gort_port} -b 0.0.0.0 -c certs -t {source_token}'
    logging.info(f'Run local proxy:{cmd}')
    run_status = subprocess.run(cmd, shell=True)

    if run_status.returncode != 0:
        logging.error("Failed to execute command")
        exit(1)

def main(args):
    
    start_boto3_client_session(args.profile, args.region.lower())
    if args.create_device_tunnel is not None:
        return open_tunnel(args.create_device_tunnel.upper())
    if args.get_device_tunnel_status is not None:
        return is_open(args.get_device_tunnel_status.upper())
    if args.rotate_device_tokens is not None:
        return rotate_access_tokens(args.rotate_device_tokens.upper())
    if args.connect_device is not None:
        return start_local_proxy_for_source(args.connect_device.upper(), args.ssh_port, args.gort_port)

    parser.print_help()

if __name__ == "__main__":
    parser = ArgumentParser(
        description="IoT Secure Tunnel Cli.",
    )
    parser.add_argument('-c','--create-device-tunnel',type=str,help='Create a new tunnel for a <device>. Returned is both a source and a destination token', dest='create_device_tunnel' )
    parser.add_argument('-g','--get-device-tunnel-status',type=str,help='Get tunnel status for a <device> - either Open, Closed, or No tunnel' , dest='get_device_tunnel_status' )
    parser.add_argument('-ro','--rotate-device-tokens',type=str,help='Rotate tokens for <device> returned a source and a destination token', dest='rotate_device_tokens' )
    parser.add_argument('-con', '--connect-device',type=str,help='Connect to a <device>. This starts the localproxy component service and creates or reconnects to an existing tunnnel', dest='connect_device' )
    parser.add_argument('-p','--profile',dest='profile',required=False,default=None,type=str,help='Specify a valid AWS profile name')
    parser.add_argument('-r','--region',dest='region',default='eu-west-1',type=str,help='Specify a valid AWS region')
    parser.add_argument('-sp','--ssh-port',dest='ssh_port',default=SSH_PORT,type=str,help='Specify a valid ssh-port')
    parser.add_argument('-gp','--gort-port',dest='gort_port',default=GORT_PORT,type=str,help='Specify a valid gort-port')

    args = parser.parse_args()

    main(args)
